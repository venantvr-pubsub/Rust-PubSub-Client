use rust_pubsub_client::idempotence::IdempotenceFilter;

#[test]
fn test_idempotence_basic() {
    let filter = IdempotenceFilter::new(100);

    assert!(filter.should_process(Some("id1")));
    assert!(!filter.should_process(Some("id1")));
    assert!(filter.should_process(Some("id2")));
}

#[test]
fn test_idempotence_capacity() {
    let filter = IdempotenceFilter::new(3);

    assert!(filter.should_process(Some("id1")));
    assert!(filter.should_process(Some("id2")));
    assert!(filter.should_process(Some("id3")));
    assert!(filter.should_process(Some("id4")));

    assert!(filter.should_process(Some("id1")));
    assert!(!filter.should_process(Some("id4")));
}

#[test]
fn test_idempotence_none() {
    let filter = IdempotenceFilter::new(10);

    assert!(filter.should_process(None));
    assert!(filter.should_process(None));
}

#[test]
fn test_idempotence_reset() {
    let filter = IdempotenceFilter::new(10);

    assert!(filter.should_process(Some("id1")));
    assert!(!filter.should_process(Some("id1")));

    filter.reset();

    assert!(filter.should_process(Some("id1")));
    assert_eq!(filter.len(), 1);
}

#[test]
fn test_idempotence_concurrent() {
    use std::sync::Arc;
    use std::thread;

    let filter = Arc::new(IdempotenceFilter::new(1000));
    let mut handles = vec![];

    for i in 0..10 {
        let filter = filter.clone();
        let handle = thread::spawn(move || {
            for j in 0..100 {
                let id = format!("thread_{}_msg_{}", i, j);
                filter.should_process(Some(&id));
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    assert_eq!(filter.len(), 1000);
}
