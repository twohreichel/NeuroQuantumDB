//! Unit tests for cluster node types.

use neuroquantum_cluster::node::{NodeRole, NodeState};

#[test]
fn test_node_role_display() {
    assert_eq!(format!("{}", NodeRole::Leader), "Leader");
    assert_eq!(format!("{}", NodeRole::Follower), "Follower");
    assert_eq!(format!("{}", NodeRole::Candidate), "Candidate");
    assert_eq!(format!("{}", NodeRole::Learner), "Learner");
}

#[test]
fn test_node_state_display() {
    assert_eq!(format!("{}", NodeState::Initializing), "Initializing");
    assert_eq!(format!("{}", NodeState::Running), "Running");
    assert_eq!(format!("{}", NodeState::Stopped), "Stopped");
}
