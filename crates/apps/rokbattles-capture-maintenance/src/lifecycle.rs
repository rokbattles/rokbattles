//! Fail-closed ordering shared by the real installer and synthetic lifecycle tests.
use std::io;

pub trait Machine {
    /// Must retain the process-wide native-open mutex until release_gate.
    fn acquire_gate(&mut self) -> io::Result<()>;
    fn mark_unavailable(&mut self) -> io::Result<()>;
    fn validate_existing_services(&mut self) -> io::Result<()>;
    fn stop_helper(&mut self) -> io::Result<()>;
    fn stop_driver(&mut self) -> io::Result<()>;
    fn require_agents_exited(&mut self) -> io::Result<()>;
    fn backup_stopped_payload(&mut self) -> io::Result<()>;
    fn validate_new_payload(&mut self) -> io::Result<()>;
    fn register_services(&mut self) -> io::Result<()>;
    fn start_driver(&mut self) -> io::Result<()>;
    fn probe_passive_open(&mut self) -> io::Result<()>;
    fn release_gate(&mut self);
    fn start_helper(&mut self) -> io::Result<()>;
    fn mark_available(&mut self) -> io::Result<()>;
    fn restore_stopped_payload(&mut self) -> io::Result<()>;
    fn remove_services(&mut self) -> io::Result<()>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    New,
    Prepared,
    Complete,
    RepairRequired,
}

pub struct Maintenance<M> {
    machine: M,
    phase: Phase,
}
impl<M: Machine> Maintenance<M> {
    pub fn new(machine: M) -> Self {
        Self { machine, phase: Phase::New }
    }
    pub fn phase(&self) -> Phase {
        self.phase
    }
    pub fn prepare(&mut self) -> io::Result<()> {
        if self.phase != Phase::New {
            return Err(invalid());
        }
        let result = (|| {
            self.machine.acquire_gate()?;
            self.machine.mark_unavailable()?;
            self.machine.validate_existing_services()?;
            self.machine.stop_helper()?;
            self.machine.stop_driver()?;
            self.machine.require_agents_exited()?;
            self.machine.backup_stopped_payload()
        })();
        self.phase = if result.is_ok() { Phase::Prepared } else { Phase::RepairRequired };
        result
    }
    pub fn commit(&mut self) -> io::Result<()> {
        if self.phase != Phase::Prepared {
            return Err(invalid());
        }
        // The installer owns the gate through the first NO_INSTALL/SNIFF/RECV_ONLY
        // open. No code copy can race an existing or new capture handle.
        let result = (|| {
            self.machine.validate_new_payload()?;
            self.machine.register_services()?;
            self.machine.start_driver()?;
            self.machine.probe_passive_open()?;
            self.machine.release_gate();
            self.machine.start_helper()?;
            self.machine.mark_available()
        })();
        self.phase = if result.is_ok() { Phase::Complete } else { Phase::RepairRequired };
        if result.is_err() {
            // Never clear the persistent marker on failure. Reacquire before any
            // rollback, including failures after helper startup.
            if self.machine.acquire_gate().is_ok()
                && self.machine.stop_helper().is_ok()
                && self.machine.stop_driver().is_ok()
            {
                let _rollback = self.machine.restore_stopped_payload();
            }
        }
        result
    }
    pub fn uninstall(&mut self) -> io::Result<()> {
        if self.phase != Phase::Prepared {
            return Err(invalid());
        }
        let result = self.machine.remove_services();
        self.phase = if result.is_ok() { Phase::Complete } else { Phase::RepairRequired };
        result
    }
}
fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, "invalid maintenance phase")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Default)]
    struct Mock {
        calls: Vec<&'static str>,
        fail: Option<&'static str>,
        gate: bool,
    }
    impl Mock {
        fn call(&mut self, name: &'static str) -> io::Result<()> {
            self.calls.push(name);
            if self.fail == Some(name) {
                Err(io::Error::other("synthetic failure"))
            } else {
                Ok(())
            }
        }
    }
    impl Machine for Mock {
        fn acquire_gate(&mut self) -> io::Result<()> {
            self.call("lock")?;
            self.gate = true;
            Ok(())
        }
        fn release_gate(&mut self) {
            self.calls.push("unlock");
            self.gate = false;
        }
        fn mark_unavailable(&mut self) -> io::Result<()> {
            self.call("block")
        }
        fn validate_existing_services(&mut self) -> io::Result<()> {
            self.call("validate-old")
        }
        fn stop_helper(&mut self) -> io::Result<()> {
            if !self.gate {
                return Err(io::Error::other("missing gate"));
            }
            self.call("stop-helper")
        }
        fn stop_driver(&mut self) -> io::Result<()> {
            if !self.gate {
                return Err(io::Error::other("missing gate"));
            }
            self.call("stop-driver")
        }
        fn require_agents_exited(&mut self) -> io::Result<()> {
            if !self.gate {
                return Err(io::Error::other("missing gate"));
            }
            self.call("agents-exited")
        }
        fn backup_stopped_payload(&mut self) -> io::Result<()> {
            self.call("backup")
        }
        fn validate_new_payload(&mut self) -> io::Result<()> {
            if !self.gate {
                return Err(io::Error::other("missing gate"));
            }
            self.call("validate-new")
        }
        fn register_services(&mut self) -> io::Result<()> {
            if !self.gate {
                return Err(io::Error::other("missing gate"));
            }
            self.call("register")
        }
        fn start_driver(&mut self) -> io::Result<()> {
            if !self.gate {
                return Err(io::Error::other("missing gate"));
            }
            self.call("start-driver")
        }
        fn probe_passive_open(&mut self) -> io::Result<()> {
            if !self.gate {
                return Err(io::Error::other("missing gate"));
            }
            self.call("probe")
        }
        fn start_helper(&mut self) -> io::Result<()> {
            if self.gate {
                return Err(io::Error::other("gate still held"));
            }
            self.call("start-helper")
        }
        fn mark_available(&mut self) -> io::Result<()> {
            self.call("unblock")
        }
        fn restore_stopped_payload(&mut self) -> io::Result<()> {
            if !self.gate {
                return Err(io::Error::other("missing gate"));
            }
            self.call("rollback")
        }
        fn remove_services(&mut self) -> io::Result<()> {
            if !self.gate {
                return Err(io::Error::other("missing gate"));
            }
            self.call("delete-services")
        }
    }
    #[test]
    fn installer_owns_gate_until_driver_probe_then_releases_before_helper_start() {
        let mut session = Maintenance::new(Mock::default());
        session.prepare().expect("prepare");
        session.commit().expect("commit");
        assert_eq!(
            session.machine.calls,
            [
                "lock",
                "block",
                "validate-old",
                "stop-helper",
                "stop-driver",
                "agents-exited",
                "backup",
                "validate-new",
                "register",
                "start-driver",
                "probe",
                "unlock",
                "start-helper",
                "unblock"
            ]
        );
        assert_eq!(session.phase(), Phase::Complete);
    }
    #[test]
    fn busy_driver_never_reaches_copy_or_registration() {
        let mut session = Maintenance::new(Mock { fail: Some("stop-driver"), ..Mock::default() });
        assert!(session.prepare().is_err());
        assert!(session.commit().is_err());
        assert!(!session.machine.calls.contains(&"backup"));
        assert_eq!(session.phase(), Phase::RepairRequired);
    }
    #[test]
    fn failed_new_signature_rolls_back_only_after_both_services_stop() {
        let mut session = Maintenance::new(Mock { fail: Some("validate-new"), ..Mock::default() });
        session.prepare().expect("prepare");
        assert!(session.commit().is_err());
        assert!(session.machine.calls.ends_with(&[
            "lock",
            "stop-helper",
            "stop-driver",
            "rollback"
        ]));
        assert!(!session.machine.calls.contains(&"register"));
        assert!(!session.machine.calls.contains(&"unblock"));
    }
    #[test]
    fn failed_helper_start_reacquires_gate_and_keeps_capture_disabled() {
        let mut session = Maintenance::new(Mock { fail: Some("start-helper"), ..Mock::default() });
        session.prepare().expect("prepare");
        assert!(session.commit().is_err());
        assert!(session.machine.calls.ends_with(&[
            "lock",
            "stop-helper",
            "stop-driver",
            "rollback"
        ]));
        assert_eq!(session.phase(), Phase::RepairRequired);
    }
    #[test]
    fn uninstall_requires_quiescence_and_never_restarts_capture() {
        let mut session = Maintenance::new(Mock::default());
        assert!(session.uninstall().is_err());
        session.prepare().expect("prepare");
        session.uninstall().expect("remove");
        assert!(session.machine.calls.ends_with(&["backup", "delete-services"]));
        assert!(!session.machine.calls.contains(&"unblock"));
    }
}
