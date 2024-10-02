use secrecy::{ExposeSecret, Secret};
use sqlx::{postgres::PgValueRef, Decode, Encode, Postgres};
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::Type, serde::Serialize, serde::Deserialize)]
#[sqlx(type_name = "role")]
#[sqlx(rename_all = "lowercase")]
pub enum UserRole {
    Admin,
    Collabolator,
}

impl ToString for UserRole {
    fn to_string(&self) -> String {
        match self {
            UserRole::Admin => "admin",
            UserRole::Collabolator => "collabolator",
        }
        .to_owned()
    }
}
pub struct UserPassword(Secret<String>);

impl UserPassword {
    pub fn get(&self) -> &Secret<String> {
        &self.0
    }
}

impl sqlx::Type<Postgres> for UserPassword {
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        <String as sqlx::Type<Postgres>>::type_info()
    }
}

impl<'q> Encode<'q, Postgres> for UserPassword {
    fn encode_by_ref(
        &self,
        buf: &mut <Postgres as sqlx::Database>::ArgumentBuffer<'q>,
    ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
        <String as Encode<Postgres>>::encode(self.0.expose_secret().clone(), buf)
    }
}

impl<'r> Decode<'r, Postgres> for UserPassword {
    fn decode(value: PgValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        let secret_str = <String as Decode<Postgres>>::decode(value)?;
        Ok(UserPassword(Secret::new(secret_str)))
    }
}

impl From<String> for UserPassword {
    fn from(password_hash: String) -> Self {
        UserPassword(Secret::new(password_hash))
    }
}

#[derive(sqlx::Type)]
pub struct User {
    pub user_id: Uuid,
    pub username: String,
    pub password_hash: UserPassword,
    pub role: UserRole,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct UserData {
    pub user_id: Uuid,
    pub username: String,
    pub role: UserRole,
}

impl From<User> for UserData {
    fn from(value: User) -> Self {
        UserData {
            user_id: value.user_id,
            username: value.username,
            role: value.role,
        }
    }
}
