use crate::{domain::subscriber_name::SubscriberName, routes::FormData};

use super::Email;

pub struct NewSubscriber {
    pub email: Email,
    pub name: SubscriberName,
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;
    fn try_from(value: FormData) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(value.name)?;
        let email = Email::parse(value.email)?;
        Ok(Self { email, name })
    }
}
