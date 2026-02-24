use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct NormalizedPlannerSummary {
    pub player_count: usize,
    pub pal_count: usize,
    pub base_assignment_count: usize,
}

pub fn empty_summary() -> NormalizedPlannerSummary {
    NormalizedPlannerSummary {
        player_count: 0,
        pal_count: 0,
        base_assignment_count: 0,
    }
}
