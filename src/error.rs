use tapo::TapoResponseError;

pub trait TapoErrorExt {
    fn is_session_timeout(&self) -> bool;
}

impl<T> TapoErrorExt for Result<T, tapo::Error> {
    fn is_session_timeout(&self) -> bool {
        match self {
            Err(tapo::Error::Tapo(TapoResponseError::SessionTimeout)) => true,
            _ => false,
        }
    }
}

impl TapoErrorExt for Option<tapo::Error> {
    fn is_session_timeout(&self) -> bool {
        if let Some(err) = self {
            err.is_session_timeout()
        } else {
            false
        }
    }
}

impl TapoErrorExt for tapo::Error {
    fn is_session_timeout(&self) -> bool {
        matches!(self, tapo::Error::Tapo(TapoResponseError::SessionTimeout))
    }
}
