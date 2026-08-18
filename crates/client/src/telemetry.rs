mod event_coalescer;


use anyhow::{Context as _, Result};
use clock::SystemClock;
use fs::Fs;
use futures::channel::mpsc;
use futures::{Future, StreamExt};
use gpui::{App, AppContext as _, BackgroundExecutor, Task};
use http_client::{self, AsyncBody, HttpClient, HttpClientWithUrl, Method, Request};
use parking_lot::Mutex;
use regex::Regex;
use release_channel::ReleaseChannel;
use settings::{Settings, SettingsStore};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs::File;
use std::io::Write;
use std::sync::LazyLock;
use std::time::Instant;
use std::{env, mem, path::PathBuf, sync::Arc, time::Duration};


pub struct TelemetrySubscription {
    pub historical_events: Result<HistoricalEvents>,
    pub queued_events: Vec<EventWrapper>,
    pub live_events: mpsc::UnboundedReceiver<EventWrapper>,
}

pub struct HistoricalEvents {
    pub events: Vec<EventWrapper>,
    pub parse_error_count: usize,
}
use util::ResultExt as _;
use worktree::{UpdatedEntriesSet, WorktreeId};

use self::event_coalescer::EventCoalescer;





#[cfg(debug_assertions)]
const MAX_QUEUE_LEN: usize = 5;

#[cfg(not(debug_assertions))]
const MAX_QUEUE_LEN: usize = 50;

#[cfg(debug_assertions)]
const FLUSH_INTERVAL: Duration = Duration::from_secs(1);

#[cfg(not(debug_assertions))]
const FLUSH_INTERVAL: Duration = Duration::from_secs(60 * 5);
static ZED_CLIENT_CHECKSUM_SEED: LazyLock<Option<Vec<u8>>> = LazyLock::new(|| {
    option_env!("ZED_CLIENT_CHECKSUM_SEED")
        .map(|s| s.as_bytes().into())
        .or_else(|| {
            env::var("ZED_CLIENT_CHECKSUM_SEED")
                .ok()
                .map(|s| s.as_bytes().into())
        })
});

pub static MINIDUMP_ENDPOINT: LazyLock<Option<String>> = LazyLock::new(|| {
    option_env!("ZED_MINIDUMP_ENDPOINT")
        .map(str::to_string)
        .or_else(|| env::var("ZED_MINIDUMP_ENDPOINT").ok())
});

pub fn should_install_crash_handler(channel: ReleaseChannel) -> bool {
    matches!(
        env::var("ZED_GENERATE_MINIDUMPS").as_deref(),
        Ok("true" | "1")
    ) || (channel != ReleaseChannel::Dev && MINIDUMP_ENDPOINT.is_some())
}

static DOTNET_PROJECT_FILES_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(global\.json|Directory\.Build\.props|.*\.(csproj|fsproj|vbproj|sln))$").unwrap()
});

pub fn os_name() -> String {
    #[cfg(target_os = "macos")]
    {
        "macOS".to_string()
    }
    #[cfg(target_os = "linux")]
    {
        format!("Linux {}", gpui::guess_compositor())
    }
    #[cfg(target_os = "freebsd")]
    {
        format!("FreeBSD {}", gpui::guess_compositor())
    }

    #[cfg(target_os = "windows")]
    {
        "Windows".to_string()
    }
}

/// Note: This might do blocking IO! Only call from background threads
pub fn os_version() -> String {
    cfg_select! {
       feature = "test-support" => {
           // MacOS branch in particular is quite slow, hence we ought to "avoid" it in tests.
           "test binary".to_owned()
       }
       target_os = "macos" => {
           static MACOS_VERSION_REGEX: LazyLock<Regex> = LazyLock::new(|| {
               Regex::new(r"(\s*\(Build [^)]*[0-9]\))").unwrap()
           });
           use objc2_foundation::NSProcessInfo;
           let process_info = NSProcessInfo::processInfo();
           let version_nsstring = process_info.operatingSystemVersionString();
           // "Version 15.6.1 (Build 24G90)" -> "15.6.1 (Build 24G90)"
           let version_string = version_nsstring.to_string().replace("Version ", "");
           // "15.6.1 (Build 24G90)" -> "15.6.1"
           // "26.0.0 (Build 25A5349a)" -> unchanged (Beta or Rapid Security Response; ends with letter)
           MACOS_VERSION_REGEX
               .replace_all(&version_string, "")
               .to_string()
       }
       any(target_os = "linux", target_os = "freebsd") => {
           use std::path::Path;

           let content = if let Ok(file) = std::fs::read_to_string(&Path::new("/etc/os-release")) {
               file
           } else if let Ok(file) = std::fs::read_to_string(&Path::new("/usr/lib/os-release")) {
               file
           } else if let Ok(file) = std::fs::read_to_string(&Path::new("/var/run/os-release")) {
               file
           } else {
               log::error!(
                   "Failed to load /etc/os-release, /usr/lib/os-release, or /var/run/os-release"
               );
               "".to_string()
           };
           util::parse_os_release(&content).unwrap_or_else(|| "unknown".to_string())
       }
       target_os = "windows" => {
           let mut info = unsafe { std::mem::zeroed() };
           let status = unsafe { windows::Wdk::System::SystemServices::RtlGetVersion(&mut info) };
           if status.is_ok() {
               semver::Version::new(
                   info.dwMajorVersion as _,
                   info.dwMinorVersion as _,
                   info.dwBuildNumber as _,
               )
               .to_string()
           } else {
               "unknown".to_string()
           }
       }
    }
}



pub fn calculate_json_checksum(json: &impl AsRef<[u8]>) -> Option<String> {
    let checksum_seed = ZED_CLIENT_CHECKSUM_SEED.as_ref()?;

    let mut summer = Sha256::new();
    summer.update(checksum_seed);
    summer.update(json);
    summer.update(checksum_seed);
    let mut checksum = String::new();
    for byte in summer.finalize().as_slice() {
        use std::fmt::Write;
        write!(&mut checksum, "{:02x}", byte).unwrap();
    }

    Some(checksum)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clock::FakeSystemClock;

    use gpui::TestAppContext;
    use http_client::FakeHttpClient;
    use std::collections::HashMap;
    
    use util::rel_path::RelPath;
    use worktree::{PathChange, ProjectEntryId, WorktreeId};

    #[gpui::test]
    async fn test_telemetry_flush_on_max_queue_size(
        executor: BackgroundExecutor,
        cx: &mut TestAppContext,
    ) {
        init_test(cx);
        let clock = Arc::new(FakeSystemClock::new());
        let http = FakeHttpClient::with_200_response();
        
        let installation_id = Some("installation_id".to_string());
        let session_id = "session_id".to_string();

        let (telemetry, first_date_time, event) = cx.update(|cx| {
            

            .max_queue_size = 4;
            

            assert!(is_empty_state());

            let first_date_time = clock.utc_now();
            let event_properties = HashMap::from_iter([(
                "test_key".to_string(),
                serde_json::Value::String("test_value".to_string()),
            )]);

            let event = FlexibleEvent {
                event_type: "test".to_string(),
                event_properties,
            };

            (telemetry, first_date_time, event)
        });

        cx.update(|_cx| {
            
            assert_eq!( 1);
            assert!();
            assert_eq!(
                .first_event_date_time,
                Some(first_date_time)
            );

            clock.advance(Duration::from_millis(100));

            
            assert_eq!( 2);
            assert!();
            assert_eq!(
                .first_event_date_time,
                Some(first_date_time)
            );

            clock.advance(Duration::from_millis(100));

            
            assert_eq!( 3);
            assert!();
            assert_eq!(
                .first_event_date_time,
                Some(first_date_time)
            );

            clock.advance(Duration::from_millis(100));

            // Adding a 4th event should cause a flush
            
        });

        // Run the spawned flush task to completion
        executor.run_until_parked();

        cx.update(|_cx| {
            assert!(is_empty_state());
        });
    }

    #[gpui::test]
    async fn test_telemetry_flush_on_flush_interval(
        executor: BackgroundExecutor,
        cx: &mut TestAppContext,
    ) {
        init_test(cx);
        let clock = Arc::new(FakeSystemClock::new());
        let http = FakeHttpClient::with_200_response();
        
        let installation_id = Some("installation_id".to_string());
        let session_id = "session_id".to_string();

        cx.update(|cx| {
            
            .max_queue_size = 4;
            

            assert!(is_empty_state());
            let first_date_time = clock.utc_now();

            let event_properties = HashMap::from_iter([(
                "test_key".to_string(),
                serde_json::Value::String("test_value".to_string()),
            )]);

            let event = FlexibleEvent {
                event_type: "test".to_string(),
                event_properties,
            };

            
            assert_eq!( 1);
            assert!();
            assert_eq!(
                .first_event_date_time,
                Some(first_date_time)
            );

            let duration = Duration::from_millis(1);

            // Test 1 millisecond before the flush interval limit is met
            executor.advance_clock(FLUSH_INTERVAL - duration);

            assert!(!is_empty_state());

            // Test the exact moment the flush interval limit is met
            executor.advance_clock(duration);

            assert!(is_empty_state());
        });
    }

    #[gpui::test]
    async fn test_report_remote_event_tags_origin(cx: &mut TestAppContext) {
        init_test(cx);
        let clock = Arc::new(FakeSystemClock::new());
        let http = FakeHttpClient::with_200_response();

        

        // Mirror what the remote server forwards: a bare `FlexibleEvent`, which
        // is the type produced by `telemetry::event!` / sent over the queue.
        let event_json = serde_json::to_string(&FlexibleEvent {
            event_type: "fs_watcher_poll".to_string(),
            event_properties: HashMap::from_iter([(
                "path".to_string(),
                serde_json::Value::String("/code/project".to_string()),
            )]),
        })
        .unwrap();

        cx.update(|_| {
            ;
        });

        let queue = ;
        assert_eq!(queue.len(), 1);
        let Event::Flexible(event) = &queue[0].event;
        assert_eq!(event.event_type, "fs_watcher_poll");
        // Original properties are preserved.
        assert_eq!(
            event.event_properties.get("path"),
            Some(&serde_json::Value::String("/code/project".to_string()))
        );
        // The remote server's OS is attached as properties, since the batch-level
        // OS describes the uploading client rather than the remote host.
        assert_eq!(
            event.event_properties.get("remote"),
            Some(&serde_json::Value::Bool(true))
        );
        assert_eq!(
            event.event_properties.get("remote_connection_type"),
            Some(&serde_json::Value::String("ssh".to_string()))
        );
        assert_eq!(
            event.event_properties.get("remote_os_name"),
            Some(&serde_json::Value::String("Linux".to_string()))
        );
        assert_eq!(
            event.event_properties.get("remote_os_version"),
            Some(&serde_json::Value::String("ubuntu 24.04".to_string()))
        );
        assert_eq!(
            event.event_properties.get("remote_architecture"),
            Some(&serde_json::Value::String("aarch64".to_string()))
        );
    }

    #[gpui::test]
    fn test_project_discovery_does_not_double_report(cx: &mut gpui::TestAppContext) {
        init_test(cx);

        let clock = Arc::new(FakeSystemClock::new());
        let http = FakeHttpClient::with_200_response();
        
        let worktree_id = 1;

        // Scan of empty worktree finds nothing
        test_project_discovery_helper( vec![], Some(vec![]), worktree_id);

        // Files added, second scan of worktree 1 finds project type
        test_project_discovery_helper(
            
            vec!["package.json"],
            Some(vec!["node"]),
            worktree_id,
        );

        // Third scan of worktree does not double report, as we already reported
        test_project_discovery_helper( vec!["package.json"], None, worktree_id);
    }

    #[gpui::test]
    fn test_pnpm_project_discovery(cx: &mut gpui::TestAppContext) {
        init_test(cx);

        let clock = Arc::new(FakeSystemClock::new());
        let http = FakeHttpClient::with_200_response();
        

        test_project_discovery_helper(
            
            vec!["package.json", "pnpm-lock.yaml"],
            Some(vec!["node", "pnpm"]),
            1,
        );
    }

    #[gpui::test]
    fn test_yarn_project_discovery(cx: &mut gpui::TestAppContext) {
        init_test(cx);

        let clock = Arc::new(FakeSystemClock::new());
        let http = FakeHttpClient::with_200_response();
        

        test_project_discovery_helper(
            
            vec!["package.json", "yarn.lock"],
            Some(vec!["node", "yarn"]),
            1,
        );
    }

    #[gpui::test]
    fn test_dotnet_project_discovery(cx: &mut gpui::TestAppContext) {
        init_test(cx);

        let clock = Arc::new(FakeSystemClock::new());
        let http = FakeHttpClient::with_200_response();
        

        // Using different worktrees, as production code blocks from reporting a
        // project type for the same worktree multiple times

        test_project_discovery_helper(
            
            vec!["global.json"],
            Some(vec!["dotnet"]),
            1,
        );
        test_project_discovery_helper(
            
            vec!["Directory.Build.props"],
            Some(vec!["dotnet"]),
            2,
        );
        test_project_discovery_helper(
            
            vec!["file.csproj"],
            Some(vec!["dotnet"]),
            3,
        );
        test_project_discovery_helper(
            
            vec!["file.fsproj"],
            Some(vec!["dotnet"]),
            4,
        );
        test_project_discovery_helper(
            
            vec!["file.vbproj"],
            Some(vec!["dotnet"]),
            5,
        );
        test_project_discovery_helper( vec!["file.sln"], Some(vec!["dotnet"]), 6);

        // Each worktree should only send a single project type event, even when
        // encountering multiple files associated with that project type
        test_project_discovery_helper(
            
            vec!["global.json", "Directory.Build.props"],
            Some(vec!["dotnet"]),
            7,
        );
    }

    // TODO:
    // Test settings
    // Update FakeHTTPClient to keep track of the number of requests and assert on it

    fn init_test(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let settings_store = SettingsStore::test(cx);
            cx.set_global(settings_store);
        });
    }

    fn is_empty_state() -> bool {
        
            && 
            && 
    }

    fn test_project_discovery_helper(
        
        file_paths: Vec<&str>,
        expected_project_types: Option<Vec<&str>>,
        worktree_id_num: usize,
    ) {
        let worktree_id = WorktreeId::from_usize(worktree_id_num);
        let entries: Vec<_> = file_paths
            .into_iter()
            .enumerate()
            .filter_map(|(i, path)| {
                Some((
                    Arc::from(RelPath::from_unix_str(path).ok()?),
                    ProjectEntryId::from_proto(i as u64 + 1),
                    PathChange::Added,
                ))
            })
            .collect();
        let updated_entries: UpdatedEntriesSet = Arc::from(entries.as_slice());

        let detected_project_types = ;

        let expected_project_types =
            expected_project_types.map(|types| types.iter().map(|&t| t.to_string()).collect());

        assert_eq!(detected_project_types, expected_project_types);
    }
}
