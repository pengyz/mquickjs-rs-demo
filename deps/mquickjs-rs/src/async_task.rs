//! Async Task Manager for RIDL async cancellation semantics
//!
//! This module implements the async task management system that supports:
//! - Cancellable tasks (default)
//! - Non-cancellable tasks (@nonCancellable)
//! - Timeout-based tasks (@timeout(ms))
//!
//! Design decisions from docs/knowledge/decision_ridl_async_cancellation.md:
//! - Default: cancellable (context drop cancels immediately)
//! - @nonCancellable: must complete even if context drops
//! - @timeout(ms): auto-cancel after timeout

use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// 完成项 - 用于 Worker 线程向 JS 主线程传递结果
#[derive(Debug)]
pub struct CompletionItem {
    /// 任务 ID
    pub task_id: u64,
    /// 结果（成功或失败）
    pub result: Result<String, String>,
}

/// Task priority based on RIDL decorators
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskPriority {
    /// Default: cancellable when context drops
    Cancellable,
    /// @nonCancellable: must complete even if context drops
    NonCancellable,
    /// @timeout(ms): auto-cancel after timeout
    Timeout(u64),
}

/// Task status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskStatus {
    /// Task is pending execution
    Pending,
    /// Task is currently running
    Running,
    /// Task completed successfully
    Completed,
    /// Task was cancelled
    Cancelled,
    /// Task timed out
    TimedOut,
}

/// Async task representation
#[derive(Debug)]
pub struct AsyncTask {
    /// Unique task ID
    pub id: u64,
    /// Task priority (from RIDL decorators)
    pub priority: TaskPriority,
    /// Current task status
    pub status: TaskStatus,
    /// Task creation time
    pub created_at: Instant,
    /// Task timeout (if Timeout priority)
    pub timeout: Option<Duration>,
    /// Whether the task has been cancelled
    pub cancelled: bool,
}

impl AsyncTask {
    /// Create a new async task
    pub fn new(id: u64, priority: TaskPriority) -> Self {
        let timeout = match &priority {
            TaskPriority::Timeout(ms) => Some(Duration::from_millis(*ms)),
            _ => None,
        };

        Self {
            id,
            priority,
            status: TaskStatus::Pending,
            created_at: Instant::now(),
            timeout,
            cancelled: false,
        }
    }

    /// Check if the task is cancellable
    pub fn is_cancellable(&self) -> bool {
        matches!(self.priority, TaskPriority::Cancellable)
    }

    /// Check if the task is non-cancellable
    pub fn is_non_cancellable(&self) -> bool {
        matches!(self.priority, TaskPriority::NonCancellable)
    }

    /// Check if the task has a timeout
    pub fn has_timeout(&self) -> bool {
        matches!(self.priority, TaskPriority::Timeout(_))
    }

    /// Check if the task has timed out
    pub fn has_timed_out(&self) -> bool {
        if let Some(timeout) = self.timeout {
            self.created_at.elapsed() >= timeout
        } else {
            false
        }
    }

    /// Cancel the task if it's cancellable or has timed out
    pub fn cancel(&mut self) -> bool {
        if (self.is_cancellable() || self.has_timeout()) && !self.cancelled {
            self.cancelled = true;
            self.status = TaskStatus::Cancelled;
            true
        } else {
            false
        }
    }

    /// Mark the task as running
    pub fn start(&mut self) {
        if self.status == TaskStatus::Pending {
            self.status = TaskStatus::Running;
        }
    }

    /// Mark the task as completed
    pub fn complete(&mut self) {
        if self.status == TaskStatus::Running {
            self.status = TaskStatus::Completed;
        }
    }

    /// Check if the task should be cancelled (for timeout tasks)
    pub fn should_cancel(&self) -> bool {
        if self.has_timeout() && self.has_timed_out() && !self.cancelled {
            return true;
        }
        false
    }
}

/// Async Task Manager
///
/// Manages async tasks with support for cancellation semantics.
/// Implements the design from docs/knowledge/decision_ridl_async_cancellation.md
pub struct AsyncTaskManager {
    /// Task counter for generating unique IDs
    next_id: AtomicU64,
    /// Active tasks
    pub(crate) tasks: Mutex<HashMap<u64, AsyncTask>>,
    /// Whether the context is being dropped
    context_dropping: std::sync::atomic::AtomicBool,
    /// 完成队列 - Worker 线程将结果放入此队列
    completion_queue: Mutex<VecDeque<CompletionItem>>,
}

impl AsyncTaskManager {
    /// Create a new AsyncTaskManager
    pub fn new() -> Self {
        Self {
            next_id: AtomicU64::new(1),
            tasks: Mutex::new(HashMap::new()),
            context_dropping: std::sync::atomic::AtomicBool::new(false),
            completion_queue: Mutex::new(VecDeque::new()),
        }
    }

    /// 推送完成项到队列（可以在任意线程调用）
    pub fn push_completion(&self, item: CompletionItem) {
        let mut queue = self.completion_queue.lock().unwrap();
        queue.push_back(item);
    }

    /// 批量弹出所有完成项（只能在 JS 主线程调用）
    pub fn drain_completions(&self) -> Vec<CompletionItem> {
        let mut queue = self.completion_queue.lock().unwrap();
        queue.drain(..).collect()
    }

    /// 检查完成队列是否为空
    pub fn has_completions(&self) -> bool {
        let queue = self.completion_queue.lock().unwrap();
        !queue.is_empty()
    }

    /// Register a new async task
    pub fn register_task(&self, priority: TaskPriority) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let task = AsyncTask::new(id, priority);
        
        let mut tasks = self.tasks.lock().unwrap();
        tasks.insert(id, task);
        
        id
    }

    /// Start a task
    pub fn start_task(&self, task_id: u64) {
        let mut tasks = self.tasks.lock().unwrap();
        if let Some(task) = tasks.get_mut(&task_id) {
            task.start();
        }
    }

    /// Complete a task
    pub fn complete_task(&self, task_id: u64) {
        let mut tasks = self.tasks.lock().unwrap();
        if let Some(task) = tasks.get_mut(&task_id) {
            task.complete();
        }
    }

    /// Cancel a specific task
    pub fn cancel_task(&self, task_id: u64) -> bool {
        let mut tasks = self.tasks.lock().unwrap();
        if let Some(task) = tasks.get_mut(&task_id) {
            task.cancel()
        } else {
            false
        }
    }

    /// Get task status
    pub fn get_task_status(&self, task_id: u64) -> Option<TaskStatus> {
        let tasks = self.tasks.lock().unwrap();
        tasks.get(&task_id).map(|t| t.status.clone())
    }

    /// Get task priority
    pub fn get_task_priority(&self, task_id: u64) -> Option<TaskPriority> {
        let tasks = self.tasks.lock().unwrap();
        tasks.get(&task_id).map(|t| t.priority.clone())
    }

    /// Check if a task is cancellable
    pub fn is_task_cancellable(&self, task_id: u64) -> bool {
        let tasks = self.tasks.lock().unwrap();
        tasks.get(&task_id).map_or(false, |t| t.is_cancellable())
    }

    /// Check if a task is non-cancellable
    pub fn is_task_non_cancellable(&self, task_id: u64) -> bool {
        let tasks = self.tasks.lock().unwrap();
        tasks.get(&task_id).map_or(false, |t| t.is_non_cancellable())
    }

    /// Get all active task IDs
    pub fn get_active_task_ids(&self) -> Vec<u64> {
        let tasks = self.tasks.lock().unwrap();
        tasks
            .iter()
            .filter(|(_, task)| {
                task.status == TaskStatus::Pending || task.status == TaskStatus::Running
            })
            .map(|(id, _)| *id)
            .collect()
    }

    /// Get all cancellable task IDs
    pub fn get_cancellable_task_ids(&self) -> Vec<u64> {
        let tasks = self.tasks.lock().unwrap();
        tasks
            .iter()
            .filter(|(_, task)| {
                task.is_cancellable()
                    && (task.status == TaskStatus::Pending || task.status == TaskStatus::Running)
            })
            .map(|(id, _)| *id)
            .collect()
    }

    /// Get all non-cancellable task IDs
    pub fn get_non_cancellable_task_ids(&self) -> Vec<u64> {
        let tasks = self.tasks.lock().unwrap();
        tasks
            .iter()
            .filter(|(_, task)| {
                task.is_non_cancellable()
                    && (task.status == TaskStatus::Pending || task.status == TaskStatus::Running)
            })
            .map(|(id, _)| *id)
            .collect()
    }

    /// Cancel all cancellable tasks
    ///
    /// This is called when the context is being dropped.
    /// Non-cancellable tasks are left running.
    pub fn cancel_all_cancellable(&self) -> Vec<u64> {
        let mut tasks = self.tasks.lock().unwrap();
        let mut cancelled_ids = Vec::new();

        for (id, task) in tasks.iter_mut() {
            if task.is_cancellable() && !task.cancelled {
                task.cancel();
                cancelled_ids.push(*id);
            }
        }

        cancelled_ids
    }

    /// Check for timed-out tasks and cancel them
    pub fn cancel_timed_out_tasks(&self) -> Vec<u64> {
        let mut tasks = self.tasks.lock().unwrap();
        let mut cancelled_ids = Vec::new();

        for (id, task) in tasks.iter_mut() {
            if task.should_cancel() {
                task.cancel();
                cancelled_ids.push(*id);
            }
        }

        cancelled_ids
    }

    /// Mark context as dropping
    ///
    /// This triggers cancellation of all cancellable tasks.
    pub fn mark_context_dropping(&self) {
        self.context_dropping.store(true, Ordering::SeqCst);
    }

    /// Check if context is dropping
    pub fn is_context_dropping(&self) -> bool {
        self.context_dropping.load(Ordering::SeqCst)
    }

    /// Get the number of active tasks (pending or running)
    pub fn active_task_count(&self) -> usize {
        let tasks = self.tasks.lock().unwrap();
        tasks
            .values()
            .filter(|task| {
                task.status == TaskStatus::Pending || task.status == TaskStatus::Running
            })
            .count()
    }

    /// Get the number of cancellable tasks (pending or running)
    pub fn cancellable_task_count(&self) -> usize {
        let tasks = self.tasks.lock().unwrap();
        tasks
            .values()
            .filter(|task| {
                task.is_cancellable()
                    && (task.status == TaskStatus::Pending || task.status == TaskStatus::Running)
            })
            .count()
    }

    /// Get the number of non-cancellable tasks (pending or running)
    pub fn non_cancellable_task_count(&self) -> usize {
        let tasks = self.tasks.lock().unwrap();
        tasks
            .values()
            .filter(|task| {
                task.is_non_cancellable()
                    && (task.status == TaskStatus::Pending || task.status == TaskStatus::Running)
            })
            .count()
    }

    /// Remove completed/cancelled/timed-out tasks
    pub fn cleanup_finished_tasks(&self) {
        let mut tasks = self.tasks.lock().unwrap();
        tasks.retain(|_, task| {
            task.status == TaskStatus::Pending || task.status == TaskStatus::Running
        });
    }
}

impl Default for AsyncTaskManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_task_priority_cancellable() {
        let manager = AsyncTaskManager::new();
        let task_id = manager.register_task(TaskPriority::Cancellable);
        
        assert!(manager.is_task_cancellable(task_id));
        assert!(!manager.is_task_non_cancellable(task_id));
        assert_eq!(manager.get_task_priority(task_id), Some(TaskPriority::Cancellable));
    }

    #[test]
    fn test_task_priority_noncancellable() {
        let manager = AsyncTaskManager::new();
        let task_id = manager.register_task(TaskPriority::NonCancellable);
        
        assert!(!manager.is_task_cancellable(task_id));
        assert!(manager.is_task_non_cancellable(task_id));
        assert_eq!(manager.get_task_priority(task_id), Some(TaskPriority::NonCancellable));
    }

    #[test]
    fn test_task_priority_timeout() {
        let manager = AsyncTaskManager::new();
        let task_id = manager.register_task(TaskPriority::Timeout(5000));
        
        assert!(!manager.is_task_cancellable(task_id));
        assert!(!manager.is_task_non_cancellable(task_id));
        assert_eq!(manager.get_task_priority(task_id), Some(TaskPriority::Timeout(5000)));
    }

    #[test]
    fn test_task_lifecycle() {
        let manager = AsyncTaskManager::new();
        let task_id = manager.register_task(TaskPriority::Cancellable);
        
        // Initial state
        assert_eq!(manager.get_task_status(task_id), Some(TaskStatus::Pending));
        
        // Start task
        manager.start_task(task_id);
        assert_eq!(manager.get_task_status(task_id), Some(TaskStatus::Running));
        
        // Complete task
        manager.complete_task(task_id);
        assert_eq!(manager.get_task_status(task_id), Some(TaskStatus::Completed));
    }

    #[test]
    fn test_cancel_cancellable_task() {
        let manager = AsyncTaskManager::new();
        let task_id = manager.register_task(TaskPriority::Cancellable);
        
        assert!(manager.cancel_task(task_id));
        assert_eq!(manager.get_task_status(task_id), Some(TaskStatus::Cancelled));
    }

    #[test]
    fn test_cancel_noncancellable_task() {
        let manager = AsyncTaskManager::new();
        let task_id = manager.register_task(TaskPriority::NonCancellable);
        
        assert!(!manager.cancel_task(task_id));
        assert_eq!(manager.get_task_status(task_id), Some(TaskStatus::Pending));
    }

    #[test]
    fn test_cancel_all_cancellable() {
        let manager = AsyncTaskManager::new();
        
        let cancellable_id = manager.register_task(TaskPriority::Cancellable);
        let noncancellable_id = manager.register_task(TaskPriority::NonCancellable);
        let timeout_id = manager.register_task(TaskPriority::Timeout(5000));
        
        let cancelled_ids = manager.cancel_all_cancellable();
        
        assert_eq!(cancelled_ids.len(), 1);
        assert!(cancelled_ids.contains(&cancellable_id));
        
        assert_eq!(manager.get_task_status(cancellable_id), Some(TaskStatus::Cancelled));
        assert_eq!(manager.get_task_status(noncancellable_id), Some(TaskStatus::Pending));
        assert_eq!(manager.get_task_status(timeout_id), Some(TaskStatus::Pending));
    }

    #[test]
    fn test_timeout_task() {
        let manager = AsyncTaskManager::new();
        let task_id = manager.register_task(TaskPriority::Timeout(50)); // 50ms timeout
        
        // Task should not be cancelled immediately
        assert!(!manager.cancel_timed_out_tasks().contains(&task_id));
        assert_eq!(manager.get_task_status(task_id), Some(TaskStatus::Pending));
        
        // Wait for timeout
        thread::sleep(Duration::from_millis(100));
        
        // Now task should be cancelled
        let cancelled_ids = manager.cancel_timed_out_tasks();
        assert!(cancelled_ids.contains(&task_id));
        assert_eq!(manager.get_task_status(task_id), Some(TaskStatus::Cancelled));
    }

    #[test]
    fn test_context_dropping() {
        let manager = AsyncTaskManager::new();
        
        assert!(!manager.is_context_dropping());
        
        manager.mark_context_dropping();
        assert!(manager.is_context_dropping());
    }

    #[test]
    fn test_active_task_count() {
        let manager = AsyncTaskManager::new();
        
        assert_eq!(manager.active_task_count(), 0);
        
        let task1 = manager.register_task(TaskPriority::Cancellable);
        let task2 = manager.register_task(TaskPriority::NonCancellable);
        
        assert_eq!(manager.active_task_count(), 2);
        
        manager.start_task(task1);
        assert_eq!(manager.active_task_count(), 2);
        
        manager.complete_task(task1);
        assert_eq!(manager.active_task_count(), 1);
        
        // Non-cancellable tasks cannot be cancelled
        assert!(!manager.cancel_task(task2));
        assert_eq!(manager.active_task_count(), 1);
        
        // Complete the non-cancellable task
        manager.start_task(task2);
        manager.complete_task(task2);
        assert_eq!(manager.active_task_count(), 0);
    }

    #[test]
    fn test_cancellable_task_count() {
        let manager = AsyncTaskManager::new();
        
        assert_eq!(manager.cancellable_task_count(), 0);
        
        let _task1 = manager.register_task(TaskPriority::Cancellable);
        let _task2 = manager.register_task(TaskPriority::NonCancellable);
        let _task3 = manager.register_task(TaskPriority::Timeout(5000));
        
        assert_eq!(manager.cancellable_task_count(), 1);
    }

    #[test]
    fn test_non_cancellable_task_count() {
        let manager = AsyncTaskManager::new();
        
        assert_eq!(manager.non_cancellable_task_count(), 0);
        
        let _task1 = manager.register_task(TaskPriority::Cancellable);
        let _task2 = manager.register_task(TaskPriority::NonCancellable);
        let _task3 = manager.register_task(TaskPriority::Timeout(5000));
        
        assert_eq!(manager.non_cancellable_task_count(), 1);
    }

    #[test]
    fn test_get_active_task_ids() {
        let manager = AsyncTaskManager::new();
        
        let task1 = manager.register_task(TaskPriority::Cancellable);
        let task2 = manager.register_task(TaskPriority::NonCancellable);
        let task3 = manager.register_task(TaskPriority::Timeout(5000));
        
        let active_ids = manager.get_active_task_ids();
        assert_eq!(active_ids.len(), 3);
        assert!(active_ids.contains(&task1));
        assert!(active_ids.contains(&task2));
        assert!(active_ids.contains(&task3));
        
        // Start and complete task1
        manager.start_task(task1);
        manager.complete_task(task1);
        
        let active_ids = manager.get_active_task_ids();
        assert_eq!(active_ids.len(), 2);
        assert!(!active_ids.contains(&task1));
        assert!(active_ids.contains(&task2));
        assert!(active_ids.contains(&task3));
    }

    #[test]
    fn test_get_cancellable_task_ids() {
        let manager = AsyncTaskManager::new();
        
        let task1 = manager.register_task(TaskPriority::Cancellable);
        let task2 = manager.register_task(TaskPriority::NonCancellable);
        let task3 = manager.register_task(TaskPriority::Timeout(5000));
        
        let cancellable_ids = manager.get_cancellable_task_ids();
        assert_eq!(cancellable_ids.len(), 1);
        assert!(cancellable_ids.contains(&task1));
        assert!(!cancellable_ids.contains(&task2));
        assert!(!cancellable_ids.contains(&task3));
    }

    #[test]
    fn test_get_non_cancellable_task_ids() {
        let manager = AsyncTaskManager::new();
        
        let task1 = manager.register_task(TaskPriority::Cancellable);
        let task2 = manager.register_task(TaskPriority::NonCancellable);
        let task3 = manager.register_task(TaskPriority::Timeout(5000));
        
        let non_cancellable_ids = manager.get_non_cancellable_task_ids();
        assert_eq!(non_cancellable_ids.len(), 1);
        assert!(!non_cancellable_ids.contains(&task1));
        assert!(non_cancellable_ids.contains(&task2));
        assert!(!non_cancellable_ids.contains(&task3));
    }

    #[test]
    fn test_cleanup_finished_tasks() {
        let manager = AsyncTaskManager::new();
        
        let task1 = manager.register_task(TaskPriority::Cancellable);
        let task2 = manager.register_task(TaskPriority::NonCancellable);
        
        // Complete task1
        manager.start_task(task1);
        manager.complete_task(task1);
        
        // Cancel task2 (non-cancellable tasks cannot be cancelled)
        assert!(!manager.cancel_task(task2));
        
        // Start and complete task2
        manager.start_task(task2);
        manager.complete_task(task2);
        
        assert_eq!(manager.active_task_count(), 0);
        
        manager.cleanup_finished_tasks();
        
        // After cleanup, tasks should be removed
        assert_eq!(manager.get_task_status(task1), None);
        assert_eq!(manager.get_task_status(task2), None);
    }

    #[test]
    fn test_concurrent_access() {
        let manager = Arc::new(AsyncTaskManager::new());
        let mut handles = vec![];
        
        // Spawn multiple threads to register tasks
        for i in 0..10 {
            let manager_clone = Arc::clone(&manager);
            let handle = thread::spawn(move || {
                let priority = if i % 2 == 0 {
                    TaskPriority::Cancellable
                } else {
                    TaskPriority::NonCancellable
                };
                manager_clone.register_task(priority);
            });
            handles.push(handle);
        }
        
        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }
        
        assert_eq!(manager.active_task_count(), 10);
    }
}
