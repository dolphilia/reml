use reml_frontend::span::Span;
use reml_frontend::streaming::StreamingState;

#[test]
fn metrics_and_checkpoints_flow() {
    let mut state = StreamingState::new();

    state.record_cache_hit();
    state.record_cache_hit();
    state.record_cache_miss();

    assert_eq!(state.metrics.cache_hit, 2);
    assert_eq!(state.metrics.cache_miss, 1);
    assert_eq!(state.metrics.replay_count, 0);

    let span = Span::new(0, 5);
    state.push_checkpoint(span);
    assert_eq!(state.checkpoints.len(), 1);
    assert!(!state.checkpoints[0].committed);
    assert_eq!(state.checkpoints[0].position, span);

    state.commit_last();
    assert!(state.checkpoints[0].committed);
}
