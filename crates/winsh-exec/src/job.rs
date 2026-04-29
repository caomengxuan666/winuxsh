//! Job control for the shell.
//!
//! Manages background jobs, foreground/background switching,
//! and job notifications.

use std::collections::HashMap;
use std::process::Child;

/// A job managed by the shell.
#[derive(Debug)]
pub struct Job {
    /// Job ID
    pub id: u32,
    /// Process ID
    pub pid: u32,
    /// The command string
    pub command: String,
    /// The child process handle
    pub child: Option<Child>,
    /// Job status
    pub status: JobStatus,
}

/// Status of a job.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobStatus {
    /// Running in background
    Running,
    /// Stopped (suspended)
    Stopped,
    /// Completed
    Done,
}

impl std::fmt::Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobStatus::Running => write!(f, "Running"),
            JobStatus::Stopped => write!(f, "Stopped"),
            JobStatus::Done => write!(f, "Done"),
        }
    }
}

/// Manages background jobs.
#[derive(Debug)]
pub struct JobManager {
    /// Active jobs
    jobs: HashMap<u32, Job>,
    /// Next job ID
    next_id: u32,
    /// Most recent job ID
    pub current_job: Option<u32>,
    /// Previous job ID
    pub previous_job: Option<u32>,
}

impl JobManager {
    /// Create a new job manager.
    pub fn new() -> Self {
        Self {
            jobs: HashMap::new(),
            next_id: 1,
            current_job: None,
            previous_job: None,
        }
    }

    /// Add a new job.
    pub fn add(&mut self, pid: u32, command: &str, child: Option<Child>) -> u32 {
        let id = self.next_id;
        self.next_id += 1;

        let job = Job {
            id,
            pid,
            command: command.to_string(),
            child,
            status: JobStatus::Running,
        };

        self.jobs.insert(id, job);
        self.previous_job = self.current_job;
        self.current_job = Some(id);
        id
    }

    /// Get a job by ID.
    pub fn get(&self, id: u32) -> Option<&Job> {
        self.jobs.get(&id)
    }

    /// Get a mutable reference to a job.
    pub fn get_mut(&mut self, id: u32) -> Option<&mut Job> {
        self.jobs.get_mut(&id)
    }

    /// Remove a job.
    pub fn remove(&mut self, id: u32) -> Option<Job> {
        let job = self.jobs.remove(&id);

        // Update current/previous job pointers
        if self.current_job == Some(id) {
            self.current_job = self.jobs.keys().max().copied();
        }
        if self.previous_job == Some(id) {
            self.previous_job = self.jobs.keys()
                .filter(|&&k| k != self.current_job.unwrap_or(0))
                .max()
                .copied();
        }

        job
    }

    /// Get the current job.
    pub fn current(&self) -> Option<&Job> {
        self.current_job.and_then(|id| self.jobs.get(&id))
    }

    /// Get the current job ID.
    pub fn current_id(&self) -> Option<u32> {
        self.current_job
    }

    /// Get the previous job.
    pub fn previous(&self) -> Option<&Job> {
        self.previous_job.and_then(|id| self.jobs.get(&id))
    }

    /// List all jobs.
    pub fn list(&self) -> Vec<&Job> {
        let mut jobs: Vec<&Job> = self.jobs.values().collect();
        jobs.sort_by_key(|j| j.id);
        jobs
    }

    /// Get the number of active jobs.
    pub fn count(&self) -> usize {
        self.jobs.len()
    }

    /// Clean up completed jobs.
    pub fn cleanup(&mut self) {
        let to_remove: Vec<u32> = self.jobs
            .iter()
            .filter_map(|(id, job)| {
                if job.status == JobStatus::Done {
                    Some(*id)
                } else {
                    match &job.child {
                        Some(child) => {
                            // Check if the child has exited
                            unsafe {
                                // On Windows, we can't easily poll, so just keep the job
                                None
                            }
                        }
                        None => Some(*id),
                    }
                }
            })
            .collect();

        for id in to_remove {
            self.remove(id);
        }
    }

    /// Check for completed jobs and print notifications.
    pub fn check_completions(&mut self) {
        let completed: Vec<(u32, String)> = self.jobs
            .iter_mut()
            .filter_map(|(id, job)| {
                if job.status == JobStatus::Done {
                    return Some((*id, job.command.clone()));
                }
                if let Some(ref mut child) = job.child {
                    match child.try_wait() {
                        Ok(Some(status)) => {
                            job.status = JobStatus::Done;
                            let code = status.code().unwrap_or(-1);
                            Some((*id, format!("Done  {} ({})", job.command, code)))
                        }
                        _ => None,
                    }
                } else {
                    None
                }
            })
            .collect();

        for (id, msg) in &completed {
            eprintln!("[{}] {}", id, msg);
        }
    }

    /// Set a job as stopped.
    pub fn stop(&mut self, id: u32) -> Result<(), String> {
        if let Some(job) = self.jobs.get_mut(&id) {
            job.status = JobStatus::Stopped;
            Ok(())
        } else {
            Err(format!("job {} not found", id))
        }
    }

    /// Resume a stopped job.
    pub fn resume(&mut self, id: u32) -> Result<(), String> {
        if let Some(job) = self.jobs.get_mut(&id) {
            job.status = JobStatus::Running;
            Ok(())
        } else {
            Err(format!("job {} not found", id))
        }
    }

    /// Move a job to foreground (wait for it).
    pub fn foreground(&mut self, id: u32) -> Result<i32, String> {
        let job = self.jobs.get_mut(&id)
            .ok_or_else(|| format!("job {} not found", id))?;

        job.status = JobStatus::Running;

        if let Some(ref mut child) = job.child {
            match child.wait() {
                Ok(status) => {
                    let code = status.code().unwrap_or(1);
                    job.status = JobStatus::Done;
                    Ok(code)
                }
                Err(e) => Err(format!("failed to wait for job: {}", e)),
            }
        } else {
            job.status = JobStatus::Done;
            Ok(0)
        }
    }

    /// Send a job to background.
    pub fn background(&mut self, id: u32) -> Result<(), String> {
        let job = self.jobs.get_mut(&id)
            .ok_or_else(|| format!("job {} not found", id))?;

        job.status = JobStatus::Running;
        Ok(())
    }

    /// Kill a job.
    pub fn kill(&mut self, id: u32) -> Result<(), String> {
        let job = self.jobs.get_mut(&id)
            .ok_or_else(|| format!("job {} not found", id))?;

        if let Some(ref mut child) = job.child {
            match child.kill() {
                Ok(_) => {
                    job.status = JobStatus::Done;
                    Ok(())
                }
                Err(e) => Err(format!("failed to kill job: {}", e)),
            }
        } else {
            job.status = JobStatus::Done;
            Ok(())
        }
    }

    /// Parse a job ID from a string (supports %, %n, %+, %-, %string).
    pub fn parse_id(&self, spec: &str) -> Result<u32, String> {
        if spec == "%" || spec == "%+" {
            self.current_id().ok_or_else(|| "no current job".to_string())
        } else if spec == "%-" {
            self.previous_job.ok_or_else(|| "no previous job".to_string())
        } else if let Some(rest) = spec.strip_prefix('%') {
            if let Ok(n) = rest.parse::<u32>() {
                if self.jobs.contains_key(&n) {
                    Ok(n)
                } else {
                    Err(format!("job {} not found", n))
                }
            } else {
                // Search by command prefix
                for job in self.jobs.values() {
                    if job.command.starts_with(rest) {
                        return Ok(job.id);
                    }
                }
                Err(format!("no job matching '{}'", rest))
            }
        } else if let Ok(n) = spec.parse::<u32>() {
            if self.jobs.contains_key(&n) {
                Ok(n)
            } else {
                Err(format!("job {} not found", n))
            }
        } else {
            Err(format!("invalid job specification: {}", spec))
        }
    }
}

impl Default for JobManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_manager_new() {
        let jm = JobManager::new();
        assert_eq!(jm.count(), 0);
    }

    #[test]
    fn test_job_add_and_get() {
        let mut jm = JobManager::new();
        let id = jm.add(1234, "sleep 10", None);
        assert!(jm.get(id).is_some());
        assert_eq!(jm.count(), 1);
    }

    #[test]
    fn test_job_remove() {
        let mut jm = JobManager::new();
        let id = jm.add(1234, "sleep 10", None);
        jm.remove(id);
        assert_eq!(jm.count(), 0);
    }

    #[test]
    fn test_job_current() {
        let mut jm = JobManager::new();
        let id1 = jm.add(1234, "cmd1", None);
        let id2 = jm.add(5678, "cmd2", None);
        assert_eq!(jm.current_id(), Some(id2));
    }

    #[test]
    fn test_job_parse_id() {
        let mut jm = JobManager::new();
        jm.add(1234, "sleep 10", None);
        jm.add(5678, "echo hello", None);

        assert!(jm.parse_id("%1").is_ok());
        assert!(jm.parse_id("%2").is_ok());
        assert!(jm.parse_id("%3").is_err());
        assert!(jm.parse_id("%echo").is_ok());
        assert!(jm.parse_id("%").is_ok());
    }
}
