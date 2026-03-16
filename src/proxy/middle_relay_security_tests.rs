use super::*;
use tokio::time::{Duration as TokioDuration, timeout};

#[test]
fn should_yield_sender_only_on_budget_with_backlog() {
    assert!(!should_yield_c2me_sender(0, true));
    assert!(!should_yield_c2me_sender(C2ME_SENDER_FAIRNESS_BUDGET - 1, true));
    assert!(!should_yield_c2me_sender(C2ME_SENDER_FAIRNESS_BUDGET, false));
    assert!(should_yield_c2me_sender(C2ME_SENDER_FAIRNESS_BUDGET, true));
}

#[tokio::test]
async fn enqueue_c2me_command_uses_try_send_fast_path() {
    let (tx, mut rx) = mpsc::channel::<C2MeCommand>(2);
    enqueue_c2me_command(
        &tx,
        C2MeCommand::Data {
            payload: Bytes::from_static(&[1, 2, 3]),
            flags: 0,
        },
    )
    .await
    .unwrap();

    let recv = timeout(TokioDuration::from_millis(50), rx.recv())
        .await
        .unwrap()
        .unwrap();
    match recv {
        C2MeCommand::Data { payload, flags } => {
            assert_eq!(payload.as_ref(), &[1, 2, 3]);
            assert_eq!(flags, 0);
        }
        C2MeCommand::Close => panic!("unexpected close command"),
    }
}

#[tokio::test]
async fn enqueue_c2me_command_falls_back_to_send_when_queue_is_full() {
    let (tx, mut rx) = mpsc::channel::<C2MeCommand>(1);
    tx.send(C2MeCommand::Data {
        payload: Bytes::from_static(&[9]),
        flags: 9,
    })
    .await
    .unwrap();

    let tx2 = tx.clone();
    let producer = tokio::spawn(async move {
        enqueue_c2me_command(
            &tx2,
            C2MeCommand::Data {
                payload: Bytes::from_static(&[7, 7]),
                flags: 7,
            },
        )
        .await
        .unwrap();
    });

    let _ = timeout(TokioDuration::from_millis(100), rx.recv())
        .await
        .unwrap();
    producer.await.unwrap();

    let recv = timeout(TokioDuration::from_millis(100), rx.recv())
        .await
        .unwrap()
        .unwrap();
    match recv {
        C2MeCommand::Data { payload, flags } => {
            assert_eq!(payload.as_ref(), &[7, 7]);
            assert_eq!(flags, 7);
        }
        C2MeCommand::Close => panic!("unexpected close command"),
    }
}

#[test]
fn desync_dedup_cache_is_bounded() {
    let _guard = desync_dedup_test_lock()
        .lock()
        .expect("desync dedup test lock must be available");
    clear_desync_dedup_for_testing();

    let now = Instant::now();
    for key in 0..DESYNC_DEDUP_MAX_ENTRIES as u64 {
        assert!(
            should_emit_full_desync(key, false, now),
            "unique keys up to cap must be tracked"
        );
    }

    assert!(
        !should_emit_full_desync(u64::MAX, false, now),
        "new key above cap must be suppressed to bound memory"
    );

    assert!(
        !should_emit_full_desync(7, false, now),
        "already tracked key inside dedup window must stay suppressed"
    );
}
