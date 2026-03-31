pub trait Env {
    fn var(&self, key: &str) -> Option<String>;
}

pub struct SystemEnv;

impl Env for SystemEnv {
    fn var(&self, key: &str) -> Option<String> {
        std::env::var(key).ok()
    }
}
