use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use sha2::{Digest, Sha512};

pub fn sha512(s: &[&str]) -> String {
    let mut hasher = Sha512::new();
    for s in s {
        hasher.update(s);
    }
    format!("{:x}", hasher.finalize())
}

pub fn random_string(len: usize) -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}

pub fn hash_pw(username: &str, pass: &str) -> String {
    crate::utils::sha512(&[&username, &pass])
}
