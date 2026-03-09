use byte_str::ByteStr;
use parking_lot::RwLock;
use std::sync::LazyLock;
use std::time::Instant;

type HashMap<K, V> = hashbrown::HashMap<K, V, ahash::RandomState>;

const SESSION_TTL_SECS: u64 = 3600;
const CLEANUP_INTERVAL_SECS: u64 = 300;

struct Entry {
    model_call_id: ByteStr,
    last_access: Instant,
}

pub struct SessionStore {
    sessions: RwLock<HashMap<String, Entry>>,
    last_cleanup: RwLock<Instant>,
}

static STORE: LazyLock<SessionStore> = LazyLock::new(|| SessionStore {
    sessions: RwLock::new(HashMap::default()),
    last_cleanup: RwLock::new(Instant::now()),
});

impl SessionStore {
    #[inline]
    pub fn global() -> &'static Self { &STORE }

    pub fn get(&self, session_id: &str) -> Option<ByteStr> {
        self.try_cleanup();
        let sessions = self.sessions.read();
        sessions.get(session_id).and_then(|entry| {
            if entry.last_access.elapsed().as_secs() < SESSION_TTL_SECS {
                Some(entry.model_call_id.clone())
            } else {
                None
            }
        })
    }

    pub fn save(&self, session_id: String, model_call_id: ByteStr) {
        let mut sessions = self.sessions.write();
        sessions.insert(session_id, Entry { model_call_id, last_access: Instant::now() });
    }

    fn try_cleanup(&self) {
        let should_cleanup = {
            let last = self.last_cleanup.read();
            last.elapsed().as_secs() >= CLEANUP_INTERVAL_SECS
        };
        if !should_cleanup {
            return;
        }
        if let Some(mut last) = self.last_cleanup.try_write() {
            *last = Instant::now();
            drop(last);
            let mut sessions = self.sessions.write();
            sessions.retain(|_, entry| entry.last_access.elapsed().as_secs() < SESSION_TTL_SECS);
        }
    }
}
