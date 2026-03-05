use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::pipeline::CommandTransaction;
use crate::workbench::{DocumentId, DocumentRevision};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
pub struct AgentSessionId(pub u64);

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
pub struct ProposalId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentRunStatus {
    Idle,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProposalStatus {
    PendingReview,
    Applied,
    Rejected,
    Stale,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub author: String,
    pub body: String,
    pub created_at_unix_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalBundle {
    pub id: ProposalId,
    pub session_id: AgentSessionId,
    pub title: String,
    pub summary: String,
    pub rationale: String,
    pub created_at_unix_ms: u64,
    pub status: ProposalStatus,
    pub transaction: CommandTransaction,
    pub preview_lines: Vec<String>,
    pub expected_revisions: BTreeMap<DocumentId, DocumentRevision>,
    pub target_documents: Vec<DocumentId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSession {
    pub id: AgentSessionId,
    pub title: String,
    pub workspace_root: Option<PathBuf>,
    pub created_at_unix_ms: u64,
    pub updated_at_unix_ms: u64,
    pub status: AgentRunStatus,
    pub messages: Vec<AgentMessage>,
    pub proposal_ids: Vec<ProposalId>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AgentWorkspaceState {
    pub sessions: BTreeMap<AgentSessionId, AgentSession>,
    pub proposals: BTreeMap<ProposalId, ProposalBundle>,
    pub active_session: Option<AgentSessionId>,
    pub active_proposal: Option<ProposalId>,
    pub composer_text: String,
    next_session_id: u64,
    next_proposal_id: u64,
}

impl Default for AgentWorkspaceState {
    fn default() -> Self {
        Self {
            sessions: BTreeMap::new(),
            proposals: BTreeMap::new(),
            active_session: None,
            active_proposal: None,
            composer_text: String::new(),
            next_session_id: 1,
            next_proposal_id: 1,
        }
    }
}

impl AgentWorkspaceState {
    pub fn allocate_session_id(&mut self) -> AgentSessionId {
        let id = AgentSessionId(self.next_session_id);
        self.next_session_id += 1;
        id
    }

    pub fn allocate_proposal_id(&mut self) -> ProposalId {
        let id = ProposalId(self.next_proposal_id);
        self.next_proposal_id += 1;
        id
    }

    pub fn pending_review_count(&self) -> usize {
        self.proposals
            .values()
            .filter(|proposal| proposal.status == ProposalStatus::PendingReview)
            .count()
    }

    pub fn ordered_sessions(&self) -> Vec<&AgentSession> {
        self.sessions.values().collect()
    }

    pub fn ordered_proposals(&self) -> Vec<&ProposalBundle> {
        let mut proposals: Vec<&ProposalBundle> = self.proposals.values().collect();
        proposals.sort_by_key(|proposal| proposal.id);
        proposals.reverse();
        proposals
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_review_count_only_counts_pending_items() {
        let mut state = AgentWorkspaceState::default();
        let session_id = state.allocate_session_id();
        let proposal_id = state.allocate_proposal_id();
        state.proposals.insert(
            proposal_id,
            ProposalBundle {
                id: proposal_id,
                session_id,
                title: "x".to_owned(),
                summary: "x".to_owned(),
                rationale: "x".to_owned(),
                created_at_unix_ms: 0,
                status: ProposalStatus::PendingReview,
                transaction: CommandTransaction {
                    source_intent: crate::pipeline::Intent::Help(
                        crate::pipeline::HelpIntent::About,
                    ),
                    commands: Vec::new(),
                    undo_policy: crate::pipeline::TxUndoPolicy::Skip,
                },
                preview_lines: Vec::new(),
                expected_revisions: BTreeMap::new(),
                target_documents: Vec::new(),
            },
        );
        let rejected_id = state.allocate_proposal_id();
        state.proposals.insert(
            rejected_id,
            ProposalBundle {
                id: rejected_id,
                session_id,
                title: "y".to_owned(),
                summary: "y".to_owned(),
                rationale: "y".to_owned(),
                created_at_unix_ms: 0,
                status: ProposalStatus::Rejected,
                transaction: CommandTransaction {
                    source_intent: crate::pipeline::Intent::Help(
                        crate::pipeline::HelpIntent::About,
                    ),
                    commands: Vec::new(),
                    undo_policy: crate::pipeline::TxUndoPolicy::Skip,
                },
                preview_lines: Vec::new(),
                expected_revisions: BTreeMap::new(),
                target_documents: Vec::new(),
            },
        );

        assert_eq!(state.pending_review_count(), 1);
    }
}
