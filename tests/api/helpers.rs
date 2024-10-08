use anyhow::bail;
use argon2::password_hash::SaltString;
use argon2::{Algorithm, Argon2, Params, PasswordHasher, Version};
use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use reqwest::Url;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockBuilder, MockServer};
use zero2prod::authentication::UserRole;
use zero2prod::configuration::{get_configuration, DatabaseSettings};
use zero2prod::email_client::EmailClient;
use zero2prod::idempotency;
use zero2prod::issue_delivery_worker::{try_execute_task, ExecutionOutcome};
use zero2prod::startup::{get_connection_pool, Application};
use zero2prod::telemetry::{get_subscriber, init_subscriber};

// Ensure that the `tracing` stack is only initialised once using `once_cell`
static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "info".to_string();
    let subscriber_name = "test".to_string();
    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::sink);
        init_subscriber(subscriber);
    };
});

pub struct TestUser {
    pub user_id: Uuid,
    pub username: String,
    pub password: String,
    pub role: UserRole,
}

impl TestUser {
    pub fn generate(role: UserRole) -> Self {
        Self {
            user_id: Uuid::new_v4(),
            username: Uuid::new_v4().to_string(),
            password: Uuid::new_v4().to_string(),
            role,
        }
    }

    async fn store(&self, pool: &PgPool) {
        let salt = SaltString::generate(&mut rand::thread_rng());
        let password_hash = Argon2::new(
            Algorithm::Argon2id,
            Version::V0x13,
            Params::new(15000, 2, 1, None).unwrap(),
        )
        .hash_password(self.password.as_bytes(), &salt)
        .unwrap()
        .to_string();

        sqlx::query!(
            "INSERT INTO users (user_id, username, password_hash, role) VALUES ($1, $2, $3, $4)",
            self.user_id,
            self.username,
            password_hash,
            self.role.to_string()
        )
        .execute(pool)
        .await
        .expect("Failed to store test user.");
    }
}

/// Confirmation links embedded in the request to the email API.
pub struct ConfirmationLinks {
    pub html: reqwest::Url,
    pub plain_text: reqwest::Url,
}

// task from issue_delivery_queue
pub struct Task {
    pub n_retries: Option<i16>,
    pub execute_after: Option<DateTime<Utc>>,
}

pub struct TestApp {
    pub address: String,
    pub port: u16,
    pub db_pool: PgPool,
    pub email_server: MockServer,
    pub admin_user: TestUser,
    pub collabolator_user: TestUser,
    pub api_client: reqwest::Client,
    pub email_client: EmailClient,
}

impl TestApp {
    pub async fn post_subscriptions(&self, body: String) -> reqwest::Response {
        self.api_client
            .post(&format!("{}/subscriptions", &self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    /// Extract the confirmation links embedded in the request to the email API.
    pub fn get_confirmation_links(&self, email_request: &wiremock::Request) -> ConfirmationLinks {
        let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();

        // Extract the link from one of the request fields.
        let get_link = |s: &str| {
            let links: Vec<_> = linkify::LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == linkify::LinkKind::Url)
                .collect();

            assert_eq!(links.len(), 1);

            let raw_link = links[0].as_str().to_owned();
            let mut confirmation_link = reqwest::Url::parse(&raw_link).unwrap();

            // Let's make sure we don't call random APIs on the web
            assert_eq!(confirmation_link.host_str().unwrap(), "127.0.0.1");
            confirmation_link.set_port(Some(self.port)).unwrap();
            confirmation_link
        };

        let html = get_link(body["HtmlBody"].as_str().unwrap());
        let plain_text = get_link(body["TextBody"].as_str().unwrap());
        ConfirmationLinks { html, plain_text }
    }

    pub async fn get_newsletter_form(&self) -> reqwest::Response {
        self.api_client
            .get(&format!("{}/admin/newsletters", &self.address))
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn get_publish_newsletter_form_html(&self) -> String {
        self.get_newsletter_form().await.text().await.unwrap()
    }

    pub async fn post_publish_newsletter(&self, body: &impl serde::Serialize) -> reqwest::Response {
        self.api_client
            .post(&format!("{}/admin/newsletters", &self.address))
            .form(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn post_login<Body>(&self, body: &Body) -> reqwest::Response
    where
        Body: serde::Serialize,
    {
        self.api_client
            .post(&format!("{}/login", &self.address))
            // This `reqwest` method makes sure that the body is URL-encoded
            // and the `Content-Type` header is set accordingly.
            .form(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn get_login_html(&self) -> String {
        self.api_client
            .get(&format!("{}/login", &self.address))
            .send()
            .await
            .expect("Failed to execute request.")
            .text()
            .await
            .unwrap()
    }

    pub async fn get_admin_dashboard(&self) -> reqwest::Response {
        self.api_client
            .get(&format!("{}/admin/dashboard", &self.address))
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn get_admin_dashboard_html(&self) -> String {
        self.get_admin_dashboard().await.text().await.unwrap()
    }

    pub async fn get_change_password(&self) -> reqwest::Response {
        self.api_client
            .get(&format!("{}/admin/password", &self.address))
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn post_change_password<Body>(&self, body: &Body) -> reqwest::Response
    where
        Body: serde::Serialize,
    {
        self.api_client
            .post(&format!("{}/admin/password", &self.address))
            .form(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn get_change_password_html(&self) -> String {
        self.get_change_password().await.text().await.unwrap()
    }

    pub async fn post_logout(&self) -> reqwest::Response {
        self.api_client
            .post(&format!("{}/admin/logout", &self.address))
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn login_with_admin_user(&self) {
        self.post_login(&serde_json::json!({
            "username": &self.admin_user.username,
            "password": &self.admin_user.password
        }))
        .await;
    }

    pub async fn login_with_collabolator_user(&self) {
        self.post_login(&serde_json::json!({
            "username": &self.collabolator_user.username,
            "password": &self.collabolator_user.password
        }))
        .await;
    }

    pub async fn dispatch_all_pending_emails(&self) -> Result<(), anyhow::Error> {
        loop {
            match try_execute_task(&self.db_pool, &self.email_client).await {
                Ok(status) => {
                    if let ExecutionOutcome::EmptyQueue = status {
                        break;
                    }
                }
                Err(err) => bail!(err),
            }
        }
        Ok(())
    }

    pub async fn count_idempotency_keys(&self) -> i64 {
        sqlx::query_scalar("SELECT COUNT(*) FROM idempotency")
            .fetch_one(&self.db_pool)
            .await
            .unwrap()
    }

    pub async fn remove_old_idempotency_keys(&self) {
        idempotency::try_delete_expired_keys(&self.db_pool)
            .await
            .unwrap()
    }

    pub async fn fetch_task(&self) -> Task {
        sqlx::query_as!(
            Task,
            "SELECT n_retries, execute_after 
            FROM issue_delivery_queue 
            LIMIT 1"
        )
        .fetch_one(&self.db_pool)
        .await
        .unwrap()
    }

    pub async fn get_invite_form(&self) -> reqwest::Response {
        self.api_client
            .get(&format!("{}/admin/collabolators", &self.address))
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn get_invite_form_html(&self) -> String {
        self.get_invite_form()
            .await
            .text()
            .await
            .expect("Cannot fetch HTML content")
    }

    pub async fn post_invite<Body: serde::Serialize>(&self, body: &Body) -> reqwest::Response {
        self.api_client
            .post(&format!("{}/admin/collabolators", &self.address))
            .form(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn get_account_activate_form<Query: serde::Serialize>(
        &self,
        query: &Query,
    ) -> reqwest::Response {
        self.api_client
            .get(&format!("{}/collabolators/activate", &self.address))
            .query(query)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn get_account_activate_form_html(&self, token: &str) -> String {
        self.get_account_activate_form(&[("token", token)])
            .await
            .text()
            .await
            .expect("Cannot fetch HTML content")
    }

    pub async fn post_account_activate<Body: serde::Serialize>(
        &self,
        body: &Body,
    ) -> reqwest::Response {
        self.api_client
            .post(&format!("{}/collabolators/activate", &self.address))
            .form(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }
}

#[allow(clippy::let_underscore_future)]
pub async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);

    let email_server = MockServer::start().await;

    // Randomise configuration to ensure test isolation
    let configuration = {
        let mut c = get_configuration().expect("Failed to read configuration.");
        // Use a different database for each test case
        c.database.database_name = Uuid::new_v4().to_string();
        // Use a random OS port
        c.application.port = 0;
        // Use the mock server as email API
        c.email_client.base_url = Url::parse(&email_server.uri()).unwrap().into();
        c
    };

    configure_database(&configuration.database).await;

    let application = Application::build(configuration.clone())
        .await
        .expect("Failed to build application.");

    // Get the port before spawning the application
    let address = format!("http://127.0.0.1:{}", application.port());
    let port = application.port();
    let _ = tokio::spawn(application.run_until_stopped());

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .cookie_store(true)
        .build()
        .unwrap();

    let test_app = TestApp {
        address,
        port,
        db_pool: get_connection_pool(&configuration.database),
        email_server,
        admin_user: TestUser::generate(UserRole::Admin),
        collabolator_user: TestUser::generate(UserRole::Collabolator),
        api_client: client,
        email_client: configuration.email_client.client(),
    };
    test_app.admin_user.store(&test_app.db_pool).await;
    test_app.collabolator_user.store(&test_app.db_pool).await;
    test_app
}

async fn configure_database(config: &DatabaseSettings) -> PgPool {
    // Create database
    let mut connection = PgConnection::connect_with(&config.without_db())
        .await
        .expect("Failed to connect to Postgres");
    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str())
        .await
        .expect("Failed to create database.");

    // Migrate database
    let connection_pool = PgPool::connect_with(config.with_db())
        .await
        .expect("Failed to connect to Postgres.");
    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate the database");

    connection_pool
}

pub fn assert_is_redirect_to(response: &reqwest::Response, location: &str) {
    assert_eq!(response.status().as_u16(), 303);
    assert_eq!(response.headers().get("Location").unwrap(), location);
}

// Short-hand for a common mocking setup
pub fn when_sending_an_email() -> MockBuilder {
    Mock::given(path("/email")).and(method("POST"))
}
