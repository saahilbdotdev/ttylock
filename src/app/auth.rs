extern crate pam;

pub fn authenticate(service: &str, username: &str, password: &str) -> bool {
    let mut auth = match pam::Client::with_password(service) {
        Ok(auth) => auth,
        Err(_) => {
            return false;
        }
    };

    auth.conversation_mut().set_credentials(username, password);

    if auth.authenticate().is_ok() && auth.open_session().is_ok() {
        return true;
    } else {
        return false;
    }
}
