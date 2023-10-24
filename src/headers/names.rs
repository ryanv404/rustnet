use std::{borrow::Cow, fmt};

impl_header_names! {
    Accept:
        "accept", b"accept";
    AcceptCharset:
        "accept-charset", b"accept-charset";
    AcceptDatetime:
        "accept-datetime", b"accept-datetime";
    AcceptEncoding:
        "accept-encoding", b"accept-encoding";
    AcceptLanguage:
        "accept-language", b"accept-language";
    AcceptPatch:
        "accept-patch", b"accept-patch";
    AcceptPost:
        "accept-post", b"accept-post";
    AcceptRanges:
        "accept-ranges", b"accept-ranges";
    AccessControlAllowCredentials:
        "access-control-allow-credentials", b"access-control-allow-credentials";
    AccessControlAllowHeaders:
        "access-control-allow-headers", b"access-control-allow-headers";
    AccessControlAllowMethods:
        "access-control-allow-methods", b"access-control-allow-methods";
    AccessControlAllowOrigin:
        "access-control-allow-origin", b"access-control-allow-origin";
    AccessControlExposeHeaders:
        "access-control-expose-headers", b"access-control-expose-headers";
    AccessControlMaxAge:
        "access-control-max-age", b"access-control-max-age";
    AccessControlRequestHeaders:
        "access-control-request-headers", b"access-control-request-headers";
    AccessControlRequestMethod:
        "access-control-request-method", b"access-control-request-method";
    Age:
        "age", b"age";
    Allow:
        "allow", b"allow";
    AltSvc:
        "alt-svc", b"alt-svc";
    Authorization:
        "authorization", b"authorization";
    CacheControl:
        "cache-control", b"cache-control";
    CacheStatus:
        "cache-status", b"cache-status";
    CdnCacheControl:
        "cdn-cache-control", b"cdn-cache-control";
    ClearSiteData:
        "clear-site-data", b"clear-site-data";
    Connection:
        "connection", b"connection";
    ContentDisposition:
        "content-disposition", b"content-disposition";
    ContentEncoding:
        "content-encoding", b"content-encoding";
    ContentLanguage:
        "content-language", b"content-language";
    ContentLength:
        "content-length", b"content-length";
    ContentLocation:
        "content-location", b"content-location";
    ContentRange:
        "content-range", b"content-range";
    ContentSecurityPolicy:
        "content-security-policy", b"content-security-policy";
    ContentSecurityPolicyReportOnly:
        "content-security-policy-report-only", b"content-security-policy-report-only";
    ContentType:
        "content-type", b"content-type";
    Cookie:
        "cookie", b"cookie";
    CrossOriginEmbedderPolicy:
        "cross-origin-embedder-policy", b"cross-origin-embedder-policy";
    CrossOriginOpenerPolicy:
        "cross-origin-opener-policy", b"cross-origin-opener-policy";
    CrossOriginResourcePolicy:
        "cross-origin-resource-policy", b"cross-origin-resource-policy";
    Date:
        "date", b"date";
    DeltaBase:
        "delta-base", b"delta-base";
    DeviceMemory:
        "device-memory", b"device-memory";
    Digest:
        "digest", b"digest";
    Dnt:
        "dnt", b"dnt";
    Etag:
        "etag", b"etag";
    Expect:
        "expect", b"expect";
    Expires:
        "expires", b"expires";
    Forwarded:
        "forwarded", b"forwarded";
    From:
        "from", b"from";
    Host:
        "host", b"host";
    IfMatch:
        "if-match", b"http2-settings";
    IfModifiedSince:
        "if-modified-since", b"if-match";
    IfNoneMatch:
        "if-none-match", b"if-modified-since";
    IfRange:
        "if-range", b"if-none-match";
    IfUnmodifiedSince:
        "if-unmodified-since", b"if-range";
    Http2Settings:
        "http2-settings", b"if-unmodified-since";
    KeepAlive:
        "keep-alive", b"keep-alive";
    LastModified:
        "last-modified", b"last-modified";
    Link:
        "link", b"link";
    Location:
        "location", b"location";
    MaxForwards:
        "max-forwards", b"max-forwards";
    Origin:
        "origin", b"origin";
    PermissionsPolicy:
        "permissions-policy", b"permissions-policy";
    Pragma:
        "pragma", b"pragma";
    Prefer:
        "prefer", b"prefer";
    ProxyAuthenticate:
        "proxy-authenticate", b"proxy-authenticate";
    ProxyAuthorization:
        "proxy-authorization", b"proxy-authorization";
    PublicKeyPins:
        "public-key-pins", b"public-key-pins";
    PublicKeyPinsReportOnly:
        "public-key-pins-report-only", b"public-key-pins-report-only";
    Purpose:
        "purpose", b"purpose";
    Range:
        "range", b"range";
    Referer:
        "referer", b"referer";
    ReferrerPolicy:
        "referrer-policy", b"referrer-policy";
    Refresh:
        "refresh", b"refresh";
    RetryAfter:
        "retry-after", b"retry-after";
    SecChUa:
        "sec-ch-ua", b"sec-ch-ua";
    SecChUaMobile:
        "sec-ch-ua-mobile", b"sec-ch-ua-mobile";
    SecChUaPlatform:
        "sec-ch-ua-platform", b"sec-ch-ua-platform";
    SaveData:
        "save-data", b"save-data";
    SecFetchDest:
        "sec-fetch-dest", b"sec-fetch-dest";
    SecFetchMode:
        "sec-fetch-mode", b"sec-fetch-mode";
    SecFetchSite:
        "sec-fetch-site", b"sec-fetch-site";
    SecFetchUser:
        "sec-fetch-user", b"sec-fetch-user";
    SecGpc:
        "sec-gpc", b"sec-gpc";
    SecWebSocketAccept:
        "sec-websocket-accept", b"sec-websocket-accept";
    SecWebSocketExtensions:
        "sec-websocket-extensions", b"sec-websocket-extensions";
    SecWebSocketKey:
        "sec-websocket-key", b"sec-websocket-key";
    SecWebSocketProtocol:
        "sec-websocket-protocol", b"sec-websocket-protocol";
    SecWebSocketVersion:
        "sec-websocket-version", b"sec-websocket-version";
    Server:
        "server", b"server";
    ServerTiming:
        "server-timing", b"server-timing";
    SetCookie:
        "set-cookie", b"set-cookie";
    Sourcemap:
        "sourcemap", b"sourcemap";
    StrictTransportSecurity:
        "strict-transport-security", b"strict-transport-security";
    Te:
        "te", b"te";
    TimingAllowOrigin:
        "timing-allow-origin", b"timing-allow-origin";
    Trailer:
        "trailer", b"trailer";
    TransferEncoding:
        "transfer-encoding", b"transfer-encoding";
    UserAgent:
        "user-agent", b"user-agent";
    Upgrade:
        "upgrade", b"upgrade";
    UpgradeInsecureRequests:
        "upgrade-insecure-requests", b"upgrade-insecure-requests";
    Vary:
        "vary", b"vary";
    Via:
        "via", b"via";
    WantDigest:
        "want-digest", b"want-digest";
    Warning:
        "warning", b"warning";
    WwwAuthenticate:
        "www-authenticate", b"www-authenticate";
    XContentTypeOptions:
        "x-content-type-options", b"x-content-type-options";
    XDnsPrefetchControl:
        "x-dns-prefetch-control", b"x-dns-prefetch-control";
    XForwardedFor:
        "x-forwarded-for", b"x-forwarded-for";
    XForwardedHost:
        "x-forwarded-host", b"x-forwarded-host";
    XForwardedProto:
        "x-forwarded-proto", b"x-forwarded-proto";
    XFrameOptions:
        "x-frame-options", b"x-frame-options";
    XPoweredBy:
        "x-powered-by", b"x-powered-by";
    XRequestId:
        "x-request-id", b"x-request-id";
    XRobotsTag:
        "x-robots-tag", b"x-robots-tag";
    XUaCompatible:
        "x-ua-compatible", b"x-ua-compatible";
    XXssProtection:
        "x-xss-protection", b"x-xss-protection";
}
