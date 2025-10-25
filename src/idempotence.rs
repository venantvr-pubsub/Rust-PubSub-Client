use parking_lot::Mutex;
use std::collections::VecDeque;

pub struct IdempotenceFilter {
    max_size: usize,
    seen_ids: Mutex<VecDeque<String>>,
}

impl IdempotenceFilter {
    pub fn new(max_size: usize) -> Self {
        Self {
            max_size,
            seen_ids: Mutex::new(VecDeque::with_capacity(max_size)),
        }
    }

    pub fn should_process(&self, message_id: Option<&str>) -> bool {
        let Some(id) = message_id else {
            return true;
        };

        let mut seen = self.seen_ids.lock();

        if seen.contains(&id.to_string()) {
            return false;
        }

        if seen.len() >= self.max_size {
            seen.pop_front();
        }

        seen.push_back(id.to_string());
        true
    }

    pub fn reset(&self) {
        self.seen_ids.lock().clear();
    }

    pub fn len(&self) -> usize {
        self.seen_ids.lock().len()
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
