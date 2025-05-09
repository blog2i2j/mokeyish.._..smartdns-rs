use nom::{
    IResult, Parser, branch::*, bytes::complete::*, character::complete::*, combinator::*,
    multi::*, sequence::*,
};

mod address_rule;
mod bool;
mod bytes;
mod client_rule;
mod cname;
mod config_for_domain;
mod domain;
mod domain_rule;
mod domain_set;
mod file_mode;
mod forward_rule;
mod glob_pattern;
mod https_record;
mod ip_alias;
mod ip_net;
mod ip_set;
mod iporset;
mod listener;
mod log_level;
mod nameserver;
mod nftset;
mod nom_recipes;
mod options;
mod path;
mod proxy_config;
mod record_type;
mod response_mode;
mod speed_mode;
mod srv;
mod svcb;

use super::*;

pub trait NomParser: Sized {
    fn parse(input: &str) -> IResult<&str, Self>;

    // fn from_str(s: &str) -> Result<Self, nom::Err<nom::error::Error<&str>>> {
    //     match Self::parse(s) {
    //         Ok((_, v)) => Ok(v),
    //         Err(err) => Err(err),
    //     }
    // }
}

impl NomParser for usize {
    #[inline]
    fn parse(input: &str) -> IResult<&str, Self> {
        map(u64, |v| v as usize).parse(input)
    }
}

impl NomParser for u64 {
    #[inline]
    fn parse(input: &str) -> IResult<&str, Self> {
        u64(input)
    }
}

impl NomParser for u8 {
    #[inline]
    fn parse(input: &str) -> IResult<&str, Self> {
        u8(input)
    }
}

impl NomParser for String {
    fn parse(input: &str) -> IResult<&str, Self> {
        map(is_not(" \t\r\n"), ToString::to_string).parse(input)
    }
}

/// one line config.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(non_camel_case_types)]
#[allow(clippy::upper_case_acronyms)]
#[allow(clippy::large_enum_variant)]
pub enum OneConfig {
    Address(AddressRule),
    AuditEnable(bool),
    AuditFile(PathBuf),
    AuditFileMode(FileMode),
    AuditNum(usize),
    AuditSize(Byte),
    BindCertFile(PathBuf),
    BindCertKeyFile(PathBuf),
    BindCertKeyPass(String),
    BlacklistIp(IpOrSet),
    BogusNxDomain(IpOrSet),
    CacheFile(PathBuf),
    CachePersist(bool),
    CacheSize(usize),
    CacheCheckpointTime(u64),
    CaFile(PathBuf),
    CaPath(PathBuf),
    ClientRule(ClientRule),
    CNAME(ConfigForDomain<CNameRule>),
    SrvRecord(ConfigForDomain<SRV>),
    GroupBegin(String),
    GroupEnd,
    HttpsRecord(ConfigForDomain<HttpsRecordRule>),
    ConfFile(PathBuf),
    DnsmasqLeaseFile(PathBuf),
    Domain(Name),
    DomainRule(ConfigForDomain<DomainRule>),
    DomainSetProvider(DomainSetProvider),
    DualstackIpAllowForceAAAA(bool),
    DualstackIpSelection(bool),
    DualstackIpSelectionThreshold(u16),
    EdnsClientSubnet(IpNet),
    ExpandPtrFromAddress(bool),
    ForceAAAASOA(bool),
    ForceHTTPSSOA(bool),
    ForceQtypeSoa(RecordType),
    ForwardRule(ForwardRule),
    HostsFile(glob::Pattern),
    IgnoreIp(IpOrSet),
    Listener(ListenerConfig),
    LocalTtl(u64),
    LogConsole(bool),
    LogNum(u64),
    LogSize(Byte),
    LogLevel(Level),
    LogFile(PathBuf),
    LogFileMode(FileMode),
    LogFilter(String),
    MaxReplyIpNum(u8),
    MdnsLookup(bool),
    NftSet(ConfigForDomain<Vec<ConfigForIP<NFTsetConfig>>>),
    NumWorkers(usize),
    PrefetchDomain(bool),
    ProxyConfig(NamedProxyConfig),
    ResolvHostname(bool),
    ResponseMode(ResponseMode),
    ServeExpired(bool),
    ServeExpiredTtl(u64),
    ServeExpiredReplyTtl(u64),
    Server(NameServerInfo),
    ServerName(Name),
    ResolvFile(PathBuf),
    RrTtl(u64),
    RrTtlMin(u64),
    RrTtlMax(u64),
    RrTtlReplyMax(u64),
    SpeedMode(Option<SpeedCheckModeList>),
    TcpIdleTime(u64),
    WhitelistIp(IpOrSet),
    User(String),
    IpSetProvider(IpSetProvider),
    IpAlias(IpAlias),
}

pub fn parse_config(input: &str) -> IResult<&str, OneConfig> {
    fn comment(input: &str) -> IResult<&str, Option<&str>> {
        opt(preceded(space1, preceded(char('#'), not_line_ending))).parse(input)
    }

    fn parse_tag<'a>(
        keyword: &'static str,
    ) -> impl Parser<&'a str, Output = (&'a str, &'a str, &'a str), Error = nom::error::Error<&'a str>>
    {
        (space0, tag_no_case(keyword), space1)
    }

    fn parse_item<'a, T: NomParser>(
        keyword: &'static str,
    ) -> impl Parser<&'a str, Output = T, Error = nom::error::Error<&'a str>> {
        preceded(parse_tag(keyword), T::parse)
    }

    let group1 = alt((
        map(parse_item("address"), OneConfig::Address),
        map(parse_item("audit-enable"), OneConfig::AuditEnable),
        map(parse_item("audit-file-mode"), OneConfig::AuditFileMode),
        map(parse_item("audit-file"), OneConfig::AuditFile),
        map(parse_item("audit-num"), OneConfig::AuditNum),
        map(parse_item("audit-size"), OneConfig::AuditSize),
        map(parse_item("bind-cert-file"), OneConfig::BindCertFile),
        map(parse_item("bind-cert-key-file"), OneConfig::BindCertKeyFile),
        map(parse_item("bind-cert-key-pass"), OneConfig::BindCertKeyPass),
        map(parse_item("bogus-nxdomain"), OneConfig::BogusNxDomain),
        map(parse_item("blacklist-ip"), OneConfig::BlacklistIp),
        map(parse_item("cache-file"), OneConfig::CacheFile),
        map(parse_item("cache-persist"), OneConfig::CachePersist),
        map(parse_item("cache-size"), OneConfig::CacheSize),
        map(
            parse_item("cache-checkpoint-time"),
            OneConfig::CacheCheckpointTime,
        ),
        map(parse_item("ca-file"), OneConfig::CaFile),
        map(parse_item("ca-path"), OneConfig::CaPath),
        map(parse_item("client-rules"), OneConfig::ClientRule),
        map(parse_item("client-rule"), OneConfig::ClientRule),
        map(parse_item("conf-file"), OneConfig::ConfFile),
    ));

    let group2 = alt((
        map(parse_item("domain-rules"), OneConfig::DomainRule),
        map(parse_item("domain-rule"), OneConfig::DomainRule),
        map(parse_item("domain-set"), OneConfig::DomainSetProvider),
        map(
            parse_item("dnsmasq-lease-file"),
            OneConfig::DnsmasqLeaseFile,
        ),
        map(
            parse_item("dualstack-ip-allow-force-AAAA"),
            OneConfig::DualstackIpAllowForceAAAA,
        ),
        map(
            parse_item("dualstack-ip-selection"),
            OneConfig::DualstackIpSelection,
        ),
        map(
            parse_item("edns-client-subnet"),
            OneConfig::EdnsClientSubnet,
        ),
        map(
            parse_item("expand-ptr-from-address"),
            OneConfig::ExpandPtrFromAddress,
        ),
        map(parse_item("force-AAAA-SOA"), OneConfig::ForceAAAASOA),
        map(parse_item("force-HTTPS-SOA"), OneConfig::ForceHTTPSSOA),
        map(parse_item("force-qtype-soa"), OneConfig::ForceQtypeSoa),
        map(parse_item("response"), OneConfig::ResponseMode),
        map(parse_item("group-begin"), OneConfig::GroupBegin),
        map(parse_tag("group-end"), |_| OneConfig::GroupEnd),
        map(parse_item("prefetch-domain"), OneConfig::PrefetchDomain),
        map(parse_item("cname"), OneConfig::CNAME),
        map(parse_item("num-workers"), OneConfig::NumWorkers),
        map(parse_item("domain"), OneConfig::Domain),
        map(parse_item("hosts-file"), OneConfig::HostsFile),
        map(parse_item("https-record"), OneConfig::HttpsRecord),
    ));

    let group3 = alt((
        map(parse_item("ignore-ip"), OneConfig::IgnoreIp),
        map(parse_item("local-ttl"), OneConfig::LocalTtl),
        map(parse_item("log-console"), OneConfig::LogConsole),
        map(parse_item("log-file-mode"), OneConfig::LogFileMode),
        map(parse_item("log-file"), OneConfig::LogFile),
        map(parse_item("log-filter"), OneConfig::LogFilter),
        map(parse_item("log-level"), OneConfig::LogLevel),
        map(parse_item("log-num"), OneConfig::LogNum),
        map(parse_item("log-size"), OneConfig::LogSize),
        map(parse_item("max-reply-ip-num"), OneConfig::MaxReplyIpNum),
        map(parse_item("mdns-lookup"), OneConfig::MdnsLookup),
        map(parse_item("nameserver"), OneConfig::ForwardRule),
        map(parse_item("proxy-server"), OneConfig::ProxyConfig),
        map(parse_item("rr-ttl-reply-max"), OneConfig::RrTtlReplyMax),
        map(parse_item("rr-ttl-min"), OneConfig::RrTtlMin),
        map(parse_item("rr-ttl-max"), OneConfig::RrTtlMax),
        map(parse_item("rr-ttl"), OneConfig::RrTtl),
        map(parse_item("resolv-file"), OneConfig::ResolvFile),
        map(parse_item("resolv-hostanme"), OneConfig::ResolvHostname),
    ));

    let group4 = alt((
        map(parse_item("response-mode"), OneConfig::ResponseMode),
        map(parse_item("server-name"), OneConfig::ServerName),
        map(parse_item("speed-check-mode"), OneConfig::SpeedMode),
        map(
            parse_item("serve-expired-reply-ttl"),
            OneConfig::ServeExpiredReplyTtl,
        ),
        map(parse_item("serve-expired-ttl"), OneConfig::ServeExpiredTtl),
        map(parse_item("serve-expired"), OneConfig::ServeExpired),
        map(parse_item("srv-record"), OneConfig::SrvRecord),
        map(parse_item("resolv-hostname"), OneConfig::ResolvHostname),
        map(parse_item("tcp-idle-time"), OneConfig::TcpIdleTime),
        map(parse_item("nftset"), OneConfig::NftSet),
        map(parse_item("user"), OneConfig::User),
    ));

    let group5 = alt((
        map(parse_item("whitelist-ip"), OneConfig::WhitelistIp),
        map(parse_item("ip-set"), OneConfig::IpSetProvider),
        map(parse_item("ip-alias"), OneConfig::IpAlias),
        map(NomParser::parse, OneConfig::Listener),
        map(NomParser::parse, OneConfig::Server),
    ));

    let group = alt((group1, group2, group3, group4, group5));

    terminated(group, comment).parse(input)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn test_nftset() {
        assert_eq!(
            parse_config("nftset /www.example.com/#4:inet#tab#dns4").unwrap(),
            (
                "",
                OneConfig::NftSet(ConfigForDomain {
                    domain: Domain::Name("www.example.com".parse().unwrap()),
                    config: vec![ConfigForIP::V4(NFTsetConfig {
                        family: "inet",
                        table: "tab".to_string(),
                        name: "dns4".to_string()
                    })]
                })
            )
        );

        assert_eq!(
            parse_config("nftset /www.example.com/#4:inet#tab#dns4 # comment 123").unwrap(),
            (
                "",
                OneConfig::NftSet(ConfigForDomain {
                    domain: Domain::Name("www.example.com".parse().unwrap()),
                    config: vec![ConfigForIP::V4(NFTsetConfig {
                        family: "inet",
                        table: "tab".to_string(),
                        name: "dns4".to_string()
                    })]
                })
            )
        );
    }

    #[test]
    fn test_parse_blacklist_ip() {
        assert_eq!(
            parse_config("blacklist-ip  243.185.187.39").unwrap(),
            (
                "",
                OneConfig::BlacklistIp(IpOrSet::Net("243.185.187.39/32".parse().unwrap()))
            )
        );

        assert_eq!(
            parse_config("blacklist-ip ip-set:name").unwrap(),
            ("", OneConfig::BlacklistIp(IpOrSet::Set("name".to_string())))
        );
    }

    #[test]
    fn test_parse_whitelist_ip() {
        assert_eq!(
            parse_config("whitelist-ip  243.185.187.39").unwrap(),
            (
                "",
                OneConfig::WhitelistIp(IpOrSet::Net("243.185.187.39/32".parse().unwrap()))
            )
        );

        assert_eq!(
            parse_config("whitelist-ip ip-set:name").unwrap(),
            ("", OneConfig::WhitelistIp(IpOrSet::Set("name".to_string())))
        );
    }

    #[test]
    fn test_parse_log_size() {
        assert_eq!(
            parse_config("log-size 1M").unwrap(),
            ("", OneConfig::LogSize("1M".parse().unwrap()))
        );
    }

    #[test]
    fn test_parse_speed_check_mode() {
        assert_eq!(
            parse_config("speed-check-mode none").unwrap(),
            ("", OneConfig::SpeedMode(Default::default()))
        );
    }

    #[test]
    fn test_parse_response_mode() {
        assert_eq!(
            parse_config("response-mode fastest-response").unwrap(),
            ("", OneConfig::ResponseMode(ResponseMode::FastestResponse))
        );
    }

    #[test]
    fn test_parse_resolv_hostname() {
        assert_eq!(
            parse_config("resolv-hostname no").unwrap(),
            ("", OneConfig::ResolvHostname(false))
        );
    }

    #[test]
    fn test_parse_domain_set() {
        assert_eq!(
            parse_config("domain-set -name outbound -file /etc/smartdns/geoip.txt").unwrap(),
            (
                "",
                OneConfig::DomainSetProvider(DomainSetProvider::File(DomainSetFileProvider {
                    name: "outbound".to_string(),
                    file: Path::new("/etc/smartdns/geoip.txt").to_path_buf(),
                    content_type: Default::default(),
                }))
            )
        );

        assert_eq!(
            parse_config("domain-set -n proxy-server -f proxy-server-list.txt").unwrap(),
            (
                "",
                OneConfig::DomainSetProvider(DomainSetProvider::File(DomainSetFileProvider {
                    name: "proxy-server".to_string(),
                    file: Path::new("proxy-server-list.txt").to_path_buf(),
                    content_type: Default::default(),
                }))
            )
        );
    }

    #[test]
    fn test_parse_domain_rule() {
        assert_eq!(
            parse_config("domain-rules /domain-set:domain-block-list/ --address #").unwrap(),
            (
                "",
                OneConfig::DomainRule(ConfigForDomain {
                    domain: Domain::Set("domain-block-list".to_string()),
                    config: DomainRule {
                        address: Some(AddressRuleValue::SOA),
                        ..Default::default()
                    }
                })
            )
        );
    }

    #[test]
    fn test_parse_ip_set() {
        assert_eq!(
            parse_config("ip-set -name name -file /path/to/file.txt").unwrap(),
            (
                "",
                OneConfig::IpSetProvider(IpSetProvider {
                    name: "name".to_string(),
                    file: Path::new("/path/to/file.txt").to_path_buf(),
                })
            )
        );
    }
}
