//! Jobs and job registry module. For database-based, timed actions.
#![allow(clippy::unused_async)] // Jobs are async.

pub mod remind;

use bonsaimq::job_registry;

job_registry!(JobRegistry, {
	Remind: "remind" => remind::job_remind,
});
