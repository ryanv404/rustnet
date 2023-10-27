use std::{fmt, str::FromStr};

use crate::{NetError, NetResult};

macro_rules! impl_header_names {
    ($( $name:ident: $output:expr, $text:expr; )+) => {
        #[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
        pub enum HeaderName {
            $( $name, )+
            Unknown(String),
        }

        impl fmt::Display for HeaderName {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.as_str())
            }
        }

        impl FromStr for HeaderName {
            type Err = NetError;

            fn from_str(s: &str) -> NetResult<Self> {
                let name = s.trim().to_lowercase();

                match name.as_str() {
                    $( $text => Ok(Self::$name), )+
                    _ => Ok(Self::Unknown(name)),
                }
            }
        }

        impl HeaderName {
            #[must_use]
            pub fn as_str(&self) -> &str {
                match self {
                    $( Self::$name => $output, )+
                    Self::Unknown(ref name) => name.as_str(),
                }
            }
        }
    };
}

impl_header_names! {
    Accept:
        "Accept", "accept";
    AcceptCharset:
        "Accept-Charset", "accept-charset";
    AcceptDatetime:
        "Accept-Datetime", "accept-datetime";
    AcceptEncoding:
        "Accept-Encoding", "accept-encoding";
    AcceptLanguage:
        "Accept-Language", "accept-language";
    AcceptPatch:
        "Accept-Patch", "accept-patch";
    AcceptPost:
        "Accept-Post", "accept-post";
    AcceptRanges:
        "Accept-Ranges", "accept-ranges";
    AccessControlAllowCredentials:
        "Access-Control-Allow-Credentials", "access-control-allow-credentials";
    AccessControlAllowHeaders:
        "Access-Control-Allow-Headers", "access-control-allow-headers";
    AccessControlAllowMethods:
        "Access-Control-Allow-Methods", "access-control-allow-methods";
    AccessControlAllowOrigin:
        "Access-Control-Allow-Origin", "access-control-allow-origin";
    AccessControlExposeHeaders:
        "Access-Control-Expose-Headers", "access-control-expose-headers";
    AccessControlMaxAge:
        "Access-Control-Max-Age", "access-control-max-age";
    AccessControlRequestHeaders:
        "Access-Control-Request-Headers", "access-control-request-headers";
    AccessControlRequestMethod:
        "Access-Control-Request-Method", "access-control-request-method";
    Age:
        "Age", "age";
    Allow:
        "Allow", "allow";
    AltSvc:
        "Alt-Svc", "alt-svc";
    Authorization:
        "Authorization", "authorization";
    CacheControl:
        "Cache-Control", "cache-control";
    CacheStatus:
        "Cache-Status", "cache-status";
    CdnCacheControl:
        "Cdn-Cache-Control", "cdn-cache-control";
    ClearSiteData:
        "Clear-Site-Data", "clear-site-data";
    Connection:
        "Connection", "connection";
    ContentDisposition:
        "Content-Disposition", "content-disposition";
    ContentEncoding:
        "Content-Encoding", "content-encoding";
    ContentLanguage:
        "Content-Language", "content-language";
    ContentLength:
        "Content-Length", "content-length";
    ContentLocation:
        "Content-Location", "content-location";
    ContentRange:
        "Content-Range", "content-range";
    ContentSecurityPolicy:
        "Content-Security-Policy", "content-security-policy";
    ContentSecurityPolicyReportOnly:
        "Content-Security-Policy-Report-Only", "content-security-policy-report-only";
    ContentType:
        "Content-Type", "content-type";
    Cookie:
        "Cookie", "cookie";
    CrossOriginEmbedderPolicy:
        "Cross-Origin-Embedder-Policy", "cross-origin-embedder-policy";
    CrossOriginOpenerPolicy:
        "Cross-Origin-Opener-Policy", "cross-origin-opener-policy";
    CrossOriginResourcePolicy:
        "Cross-Origin-Resource-Policy", "cross-origin-resource-policy";
    Date:
        "Date", "date";
    DeltaBase:
        "Delta-Base", "delta-base";
    DeviceMemory:
        "Device-Memory", "device-memory";
    Digest:
        "Digest", "digest";
    Dnt:
        "Dnt", "dnt";
    Etag:
        "Etag", "etag";
    Expect:
        "Expect", "expect";
    Expires:
        "Expires", "expires";
    Forwarded:
        "Forwarded", "forwarded";
    From:
        "From", "from";
    Host:
        "Host", "host";
    IfMatch:
        "If-Match", "http2-settings";
    IfModifiedSince:
        "If-Modified-Since", "if-match";
    IfNoneMatch:
        "If-None-Match", "if-modified-since";
    IfRange:
        "If-Range", "if-none-match";
    IfUnmodifiedSince:
        "If-Unmodified-Since", "if-range";
    Http2Settings:
        "Http2-Settings", "if-unmodified-since";
    KeepAlive:
        "Keep-Alive", "keep-alive";
    LastModified:
        "Last-Modified", "last-modified";
    Link:
        "Link", "link";
    Location:
        "Location", "location";
    MaxForwards:
        "Max-Forwards", "max-forwards";
    Origin:
        "Origin", "origin";
    PermissionsPolicy:
        "Permissions-Policy", "permissions-policy";
    Pragma:
        "Pragma", "pragma";
    Prefer:
        "Prefer", "prefer";
    ProxyAuthenticate:
        "Proxy-Authenticate", "proxy-authenticate";
    ProxyAuthorization:
        "Proxy-Authorization", "proxy-authorization";
    PublicKeyPins:
        "Public-Key-Pins", "public-key-pins";
    PublicKeyPinsReportOnly:
        "Public-Key-Pins-Report-Only", "public-key-pins-report-only";
    Purpose:
        "Purpose", "purpose";
    Range:
        "Range", "range";
    Referer:
        "Referer", "referer";
    ReferrerPolicy:
        "Referrer-Policy", "referrer-policy";
    Refresh:
        "Refresh", "refresh";
    RetryAfter:
        "Retry-After", "retry-after";
    SecChUa:
        "Sec-Ch-Ua", "sec-ch-ua";
    SecChUaMobile:
        "Sec-Ch-Ua-Mobile", "sec-ch-ua-mobile";
    SecChUaPlatform:
        "Sec-Ch-Ua-Platform", "sec-ch-ua-platform";
    SaveData:
        "Save-Data", "save-data";
    SecFetchDest:
        "Sec-Fetch-Dest", "sec-fetch-dest";
    SecFetchMode:
        "Sec-Fetch-Mode", "sec-fetch-mode";
    SecFetchSite:
        "Sec-Fetch-Site", "sec-fetch-site";
    SecFetchUser:
        "Sec-Fetch-User", "sec-fetch-user";
    SecGpc:
        "Sec-Gpc", "sec-gpc";
    SecWebSocketAccept:
        "Sec-Websocket-Accept", "sec-websocket-accept";
    SecWebSocketExtensions:
        "Sec-Websocket-Extensions", "sec-websocket-extensions";
    SecWebSocketKey:
        "Sec-Websocket-Key", "sec-websocket-key";
    SecWebSocketProtocol:
        "Sec-Websocket-Protocol", "sec-websocket-protocol";
    SecWebSocketVersion:
        "Sec-Websocket-Version", "sec-websocket-version";
    Server:
        "Server", "server";
    ServerTiming:
        "Server-Timing", "server-timing";
    SetCookie:
        "Set-Cookie", "set-cookie";
    Sourcemap:
        "Sourcemap", "sourcemap";
    StrictTransportSecurity:
        "Strict-Transport-Security", "strict-transport-security";
    Te:
        "Te", "te";
    TimingAllowOrigin:
        "Timing-Allow-Origin", "timing-allow-origin";
    Trailer:
        "Trailer", "trailer";
    TransferEncoding:
        "Transfer-Encoding", "transfer-encoding";
    UserAgent:
        "User-Agent", "user-agent";
    Upgrade:
        "Upgrade", "upgrade";
    UpgradeInsecureRequests:
        "Upgrade-Insecure-Requests", "upgrade-insecure-requests";
    Vary:
        "Vary", "vary";
    Via:
        "Via", "via";
    WantDigest:
        "Want-Digest", "want-digest";
    Warning:
        "Warning", "warning";
    WwwAuthenticate:
        "Www-Authenticate", "www-authenticate";
    XContentTypeOptions:
        "X-Content-Type-Options", "x-content-type-options";
    XDnsPrefetchControl:
        "X-Dns-Prefetch-Control", "x-dns-prefetch-control";
    XForwardedFor:
        "X-Forwarded-For", "x-forwarded-for";
    XForwardedHost:
        "X-Forwarded-Host", "x-forwarded-host";
    XForwardedProto:
        "X-Forwarded-Proto", "x-forwarded-proto";
    XFrameOptions:
        "X-Frame-Options", "x-frame-options";
    XPoweredBy:
        "X-Powered-By", "x-powered-by";
    XRequestId:
        "X-Request-Id", "x-request-id";
    XRobotsTag:
        "X-Robots-Tag", "x-robots-tag";
    XUaCompatible:
        "X-Ua-Compatible", "x-ua-compatible";
    XXssProtection:
        "X-Xss-Protection", "x-xss-protection";
}
