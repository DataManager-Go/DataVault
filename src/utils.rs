use sha2::{Digest, Sha512};

pub fn sha512(s: &[&str]) -> String {
    let mut hasher = Sha512::new();
    for s in s {
        hasher.update(s);
    }
    format!("{:x}", hasher.finalize())
}
