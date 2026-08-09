use parking_lot::Mutex;
use std::collections::{HashSet, VecDeque};
use std::sync::Arc;

// Les identifiants sont stockés une seule fois et partagés entre les deux structures :
// `order` fixe l'ordre d'éviction (FIFO), `index` répond aux tests d'appartenance en O(1).
#[derive(Default)]
struct Seen {
    order: VecDeque<Arc<str>>,
    index: HashSet<Arc<str>>,
}

pub struct IdempotenceFilter {
    max_size: usize,
    seen: Mutex<Seen>,
}

impl IdempotenceFilter {
    pub fn new(max_size: usize) -> Self {
        Self {
            max_size,
            seen: Mutex::new(Seen {
                order: VecDeque::with_capacity(max_size),
                index: HashSet::with_capacity(max_size),
            }),
        }
    }

    /// Retourne `true` si le message doit être traité, `false` s'il a déjà été vu.
    /// Enregistre l'identifiant au passage.
    ///
    /// La recherche se faisait auparavant par `VecDeque::contains`, c'est-à-dire un balayage
    /// linéaire de la file — jusqu'à `max_size` comparaisons de chaînes **pour chaque message
    /// reçu**, sur le chemin critique du client. Chaque appel allouait en plus un `String`
    /// temporaire via `id.to_string()`, y compris quand la réponse était immédiatement « déjà vu ».
    /// Le `HashSet` ramène la recherche à O(1) et l'identifiant n'est alloué que s'il est retenu.
    pub fn should_process(&self, message_id: Option<&str>) -> bool {
        let Some(id) = message_id else {
            return true;
        };

        let mut seen = self.seen.lock();

        if seen.index.contains(id) {
            return false;
        }

        // Éviction FIFO : on retire l'entrée la plus ancienne une fois la capacité atteinte.
        if seen.order.len() >= self.max_size {
            if let Some(oldest) = seen.order.pop_front() {
                seen.index.remove(&oldest);
            }
        }

        let id: Arc<str> = Arc::from(id);
        seen.order.push_back(Arc::clone(&id));
        seen.index.insert(id);
        true
    }

    pub fn reset(&self) {
        let mut seen = self.seen.lock();
        seen.order.clear();
        seen.index.clear();
    }

    pub fn len(&self) -> usize {
        self.seen.lock().order.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_idempotence_filter() {
        let filter = IdempotenceFilter::new(3);

        assert!(filter.should_process(Some("id1")));
        assert!(!filter.should_process(Some("id1")));
        assert!(filter.should_process(Some("id2")));
        assert!(filter.should_process(Some("id3")));
        assert!(filter.should_process(Some("id4")));
        assert!(filter.should_process(Some("id1")));
    }

    #[test]
    fn test_none_message_id() {
        let filter = IdempotenceFilter::new(10);
        assert!(filter.should_process(None));
        assert!(filter.should_process(None));
    }
}
