#![allow(dead_code)]
// #![feature(test)]

use cli::*;
use config::NameServerInfo;
use dns_url::DnsUrl;
use std::str::FromStr;

mod api;
mod app;
mod cli;
mod collections;
mod config;
mod dns;
mod dns_client;
mod dns_conf;
mod dns_error;
mod dns_mw;
mod dns_mw_addr;
mod dns_mw_audit;
mod dns_mw_bogus;
mod dns_mw_cache;
mod dns_mw_cname;
mod dns_mw_dns64;
mod dns_mw_dnsmasq;
mod dns_mw_dualstack;
mod dns_mw_hosts;
#[cfg(all(feature = "nft", target_os = "linux"))]
mod dns_mw_nftset;
mod dns_mw_ns;
mod dns_mw_zone;
mod dns_rule;
mod dns_url;
mod dnsmasq;
mod error;
mod ffi;
mod infra;
mod libdns;
mod log;
mod preset_ns;
mod proxy;
#[cfg(feature = "resolve-cli")]
mod resolver;
mod rustls;
mod server;
#[cfg(feature = "service")]
mod service;
mod third_ext;
#[cfg(feature = "self-update")]
mod updater;

use error::Error;
use infra::middleware;

use crate::{
    dns_client::DnsClient,
    dns_conf::RuntimeConfig,
    infra::process_guard::ProcessGuardError,
    log::{error, info, warn},
};

fn banner() {
    info!("");
    info!(r#"     _____                      _       _____  _   _  _____ "#);
    info!(r#"    / ____|                    | |     |  __ \| \ | |/ ____|"#);
    info!(r#"   | (___  _ __ ___   __ _ _ __| |_    | |  | |  \| | (___  "#);
    info!(r#"    \___ \| '_ ` _ \ / _` | '__| __|   | |  | | . ` |\___ \ "#);
    info!(r#"    ____) | | | | | | (_| | |  | |_    | |__| | |\  |____) |"#);
    info!(r#"   |_____/|_| |_| |_|\__,_|_|   \__|   |_____/|_| \_|_____/ "#);
    info!("");
}

/// The app name
const NAME: &str = "SmartDNS";

include!(concat!(env!("OUT_DIR"), "/build_time_vars.rs"));

/// The default configuration.
const DEFAULT_CONF: &str = include_str!("../etc/smartdns/smartdns.conf");

#[cfg(not(windows))]
fn main() {
    Cli::parse().run();
}

#[cfg(windows)]
fn main() -> windows_service::Result<()> {
    if matches!(std::env::args().next_back(), Some(flag) if flag == "--ws7642ea814a90496daaa54f2820254f12")
    {
        return service::windows::run();
    }

    Cli::parse().run();
    Ok(())
}

impl Cli {
    #[inline]
    pub fn run(self) {
        let _guard = self.log_level().map(log::console);

        match self.command {
            Commands::Run {
                directory,
                conf,
                pid,
                ..
            } => {
                let _guard = pid
                    .map(|pid| {
                        use infra::process_guard;
                        match process_guard::create(pid) {
                            Ok(guard) => Some(guard),
                            Err(err @ ProcessGuardError::AlreadyRunning(_)) => {
                                panic!("{}", err)
                            }
                            Err(err) => {
                                error!("{}", err);
                                None
                            }
                        }
                    })
                    .unwrap_or_default();
                hello_starting();
                let cfg = RuntimeConfig::load(directory, conf);

                cfg.summary();

                #[cfg(target_os = "linux")]
                match cfg.user() {
                    Some(user) => run_user::with(user, None).expect("switch user failed"),
                    None => run_user::try_drop_privs(),
                }
                app::serve(cfg);
                good_bye();
            }
            #[cfg(feature = "service")]
            Commands::Service {
                command: service_command,
            } => {
                use ServiceCommands::*;
                let sm = crate::service::service_manager();
                let output = match service_command {
                    Install => sm.install(),
                    Uninstall { purge } => sm.uninstall(purge, false),
                    Start => sm.start(),
                    Stop => sm.stop(),
                    Restart => sm.restart(),
                    Status => match sm.status() {
                        Ok(status) => {
                            let out = match status {
                                service::ServiceStatus::Running(out) => Some(out),
                                service::ServiceStatus::Dead(out) => Some(out),
                                service::ServiceStatus::Unknown => None,
                            };
                            if let Some(out) = out {
                                if let Ok(out) = String::from_utf8(out.stdout) {
                                    print!("{out}");
                                } else {
                                    warn!("get service status failed.");
                                }
                            }
                            Ok(())
                        }
                        Err(err) => Err(err),
                    },
                };

                if let Err(err) = output {
                    match err.kind() {
                        std::io::ErrorKind::PermissionDenied => {
                            #[cfg(windows)]
                            log::error!("{}. requires administrator privileges", err);
                            #[cfg(unix)]
                            log::error!("{}. requires root privileges", err);
                        }
                        _ => log::error!("{}", err),
                    }
                }
            }
            #[cfg(not(feature = "service"))]
            Commands::Service { command: _ } => {
                warn!("please enable `service` feature")
            }
            Commands::Test { direcory, conf } => {
                RuntimeConfig::load(direcory, conf);
            }
            #[cfg(feature = "self-update")]
            Commands::Update { yes, version } => {
                updater::update(yes, version.as_deref()).unwrap();
            }
            #[cfg(feature = "resolve-cli")]
            Commands::Resolve(command) => {
                drop(_guard);
                command.execute();
            }
            #[cfg(all(feature = "resolve-cli", any(unix, windows)))]
            Commands::Symlink { link } => {
                let original = std::env::current_exe().expect("failed to get current exe path");
                if link.exists() {
                    println!("link already exists");
                    return;
                }

                #[cfg(unix)]
                let res = std::os::unix::fs::symlink(original, link);

                #[cfg(windows)]
                let res = std::os::windows::fs::symlink_file(original, link);

                match res {
                    Ok(()) => println!("symlink created"),
                    Err(err) => println!("failed to create symlink, {err}"),
                }
            }
            #[allow(unreachable_patterns)]
            _ => {
                unimplemented!()
            }
        }
    }
}

#[inline]
fn hello_starting() {
    info!("{} 🐋 {} starting", NAME, BUILD_VERSION);
}

#[inline]
fn good_bye() {
    info!("{} {} shutdown", crate::NAME, crate::BUILD_VERSION);
}

impl RuntimeConfig {
    pub async fn create_dns_client(&self) -> DnsClient {
        let servers = self.servers();
        let ca_path = self.ca_path();
        let ca_file = self.ca_file();
        let proxies = self.proxies().clone();

        let mut builder = DnsClient::builder();

        #[cfg(feature = "mdns")]
        if self.mdns_lookup() {
            use crate::libdns::proto::multicast::{MDNS_IPV4, MDNS_IPV6};
            let mdns_servers = [*MDNS_IPV4, *MDNS_IPV6]
                .into_iter()
                .map(|ip| format!("mdns://{ip}"))
                .flat_map(|s| DnsUrl::from_str(&s).ok())
                .map(|url| {
                    let mut config = NameServerInfo::from(url);
                    config.group = vec!["mdns".to_string()];
                    config.exclude_default_group = true;
                    config
                })
                .collect::<Vec<_>>();
            builder = builder.add_servers(mdns_servers.to_vec());
        }
        builder = builder.add_servers(servers.to_vec());
        if let Some(path) = ca_path {
            builder = builder.with_ca_path(path.to_owned());
        }
        if let Some(file) = ca_file {
            builder = builder.with_ca_path(file.to_owned());
        }
        if let Some(subnet) = self.edns_client_subnet() {
            builder = builder.with_client_subnet(subnet);
        }
        builder = builder.with_proxies(proxies);
        builder.build().await
    }
}

mod signal {
    use std::sync::atomic::{AtomicBool, Ordering};

    static TERMINATING: AtomicBool = AtomicBool::new(false);

    pub async fn terminate() -> std::io::Result<()> {
        use tokio::signal::ctrl_c;

        #[cfg(unix)]
        {
            use tokio::signal::unix::{SignalKind, signal};
            match signal(SignalKind::terminate()) {
                Ok(mut terminate) => tokio::select! {
                    _ = terminate.recv() => SignalKind::terminate(),
                    _ = ctrl_c() => SignalKind::interrupt()
                },
                _ => {
                    ctrl_c().await?;
                    SignalKind::interrupt()
                }
            };
        }

        #[cfg(not(unix))]
        {
            ctrl_c().await?;
        }

        if !TERMINATING.load(Ordering::Relaxed) {
            TERMINATING.store(true, Ordering::Relaxed);
            super::info!("terminating...");
        }

        Ok(())
    }
}

#[cfg(target_os = "linux")]
mod run_user {
    use std::{collections::HashSet, io};

    use crate::log;
    use caps::{
        CapSet::{Effective, Permitted},
        Capability::{self, CAP_NET_ADMIN, CAP_NET_BIND_SERVICE, CAP_NET_BROADCAST, CAP_NET_RAW},
        securebits::set_keepcaps,
    };
    use users::{
        get_current_gid, get_current_uid, get_effective_gid, get_effective_uid, get_group_by_name,
        get_user_by_name,
        switch::{set_current_gid, set_current_uid},
    };
    use uzers as users;

    pub static DEFAULT_USER: &str = "nobody";
    pub static DEFAULT_GROUP: &str = "nobody";

    pub fn with(username: &str, groupname: Option<&str>) -> io::Result<()> {
        let mut caps = HashSet::new();
        caps.insert(CAP_NET_ADMIN); // nftset
        caps.insert(CAP_NET_BIND_SERVICE); // bind
        caps.insert(CAP_NET_BROADCAST); // mdns
        caps.insert(CAP_NET_RAW); // ping
        switch_user(username, groupname, &caps)
    }

    pub fn try_drop_privs() {
        if let Err(err) = with(DEFAULT_USER, Some(DEFAULT_GROUP)) {
            log::error!("failed to drop privs: {}", err);
        }
    }

    #[inline]
    fn switch_user(
        username: &str,
        groupname: Option<&str>,
        caps: &HashSet<Capability>,
    ) -> io::Result<()> {
        let (uid, gid, euid, egid) = (
            get_current_uid(),
            get_current_gid(),
            get_effective_uid(),
            get_effective_gid(),
        );

        if uid == 0 || euid == 0 {
            log::info!(
                "running as root: {uid}, gid: {gid} (euid: {euid}, egid: {egid})...dropping privileges."
            );
        } else {
            return Ok(()); // already running as non-root, nothing to do.
        }

        let user = get_user_by_name(username);
        let Some(user) = user else {
            return Err(io::Error::other(format!("User {username} not found")));
        };

        let group = groupname.map(get_group_by_name).unwrap_or_default();

        let uid = user.uid();
        let gid = group
            .map(|g| g.gid())
            .unwrap_or_else(|| user.primary_group_id());

        keepcaps()?;
        set_gid(gid)?;
        set_uid(uid)?;

        let (uid, gid, euid, egid) = (
            get_current_uid(),
            get_current_gid(),
            get_effective_uid(),
            get_effective_gid(),
        );

        set_caps(caps)?;

        log::info!("now running as uid: {uid}, gid: {gid} (euid: {euid}, egid: {egid})");

        Ok(())
    }

    #[inline]
    fn set_gid(gid: u32) -> io::Result<()> {
        set_current_gid(gid)
            .map_err(|err| io::Error::other(format!("Failed to set gid: {gid}, {err}")))
    }

    #[inline]
    fn set_uid(uid: u32) -> io::Result<()> {
        set_current_uid(uid)
            .map_err(|err| io::Error::other(format!("Failed to set uid: {uid}, {err}")))
    }

    #[inline]
    fn set_caps(caps: &caps::CapsHashSet) -> io::Result<()> {
        caps::set(None, Effective, caps)
            .and(caps::set(None, Permitted, caps))
            .map_err(|err| io::Error::other(format!("Failed to set capabilities: {err}")))
    }

    #[inline]
    fn keepcaps() -> io::Result<()> {
        set_keepcaps(true).map_err(|err| io::Error::other(format!("Failed to set keepcaps: {err}")))
    }
}
