//! Task details panel component.

use crate::models::Task;
use yew::prelude::*;

/// Props for TaskDetailsPanel.
#[derive(Properties, PartialEq)]
pub struct TaskDetailsPanelProps {
    pub task: Task,
    pub on_close: Callback<()>,
}

/// Detailed view of a single task.
#[function_component(TaskDetailsPanel)]
pub fn task_details_panel(props: &TaskDetailsPanelProps) -> Html {
    let close_callback = props.on_close.clone();
    
    html! {
        <div class="task-details-panel">
            <div class="panel-header">
                <h2>{"Task Details"}</h2>
                <button class="close-button" onclick={move |_| close_callback.emit(())}>
                    {"✕"}
                </button>
            </div>
            
            <div class="task-details-content">
                <div class="detail-row">
                    <span class="detail-label">{ "Task ID:" }</span>
                    <span class="detail-value">{ &props.task.id }</span>
                </div>
                
                <div class="detail-row">
                    <span class="detail-label">{ "Description:" }</span>
                    <span class="detail-value">{ &props.task.description }</span>
                </div>
                
                <div class="detail-row">
                    <span class="detail-label">{ "Status:" }</span>
                    <span class="detail-value status-{ format!("{:?}", props.task.status).to_lowercase() }>
                        { format!("{:?}", props.task.status) }
                    </span>
                </div>
                
                <div class="detail-row">
                    <span class="detail-label">{ "Priority:" }</span>
                    <span class="detail-value">{ props.task.priority }</span>
                </div>
                
                if let Some(agent) = &props.task.assigned_agent {
                    <div class="detail-row">
                        <span class="detail-label">{ "Assigned Agent:" }</span>
                        <span class="detail-value">{ agent }</span>
                    </div>
                }
                
                if let Some(deadline) = props.task.deadline {
                    <div class="detail-row">
                        <span class="detail-label">{ "Deadline:" }</span>
                        <span class="detail-value">{ format!("{}s from now", deadline) }</span>
                    </div>
                }
                
                if !props.task.dependencies.is_empty() {
                    <div class="detail-row">
                        <span class="detail-label">{ "Dependencies:" }</span>
                        <div class="dependency-tags">
                            { for props.task.dependencies.iter().map(|dep| {
                                html! {
                                    <span class="tag">{ dep }</span>
                                }
                            }) }
                        </div>
                    </div>
                }
                
                <div class="detail-row">
                    <span class="detail-label">{ "Required Capabilities:" }</span>
                    <div class="capability-tags">
                        { for props.task.required_capabilities.iter().map(|cap| {
                            html! {
                                <span class="tag capability">{ cap }</span>
                            }
                        }) }
                    </div>
                </div>
                
                <div class="detail-row">
                    <span class="detail-label">{ "Estimated Duration:" }</span>
                    <span class="detail-value">
                        { format!("{}s", props.task.estimated_duration_secs) }
                    </span>
                </div>
                
                <div class="task-actions">
                    <button class="btn btn-primary">
                        { "View Progress" }
                    </button>
                    if props.task.status == models::TaskStatus::Pending || props.task.status == models::TaskStatus::Assigned {
                        <button class="btn btn-warning">
                            { "Reassign" }
                        </button>
                        <button class="btn btn-danger">
                            { "Cancel" }
                        </button>
                    }
                </div>
            </div>
        </div>
    }
}
