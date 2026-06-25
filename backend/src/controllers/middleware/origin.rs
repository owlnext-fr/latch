//! Garde CSRF complémentaire au SameSite (contrat §4, §9.6). Toute mutation admin
//! doit présenter un `Origin` (ou `Referer` en repli) same-origin. Sinon 403.
//!
//! Le middleware compare l'hôte extrait de `Origin` (ou `Referer`) à l'hôte du
//! header `Host` de la requête. Un mismatch ou l'absence des deux → 403 Forbidden.
//!
//! Choix d'implémentation : le 403 est produit via
//! `(StatusCode::FORBIDDEN, ...).into_response()` retourné comme `Ok(...)` plutôt
//! que via `loco_rs::Error`. Raisons :
//! - `loco_rs::Error::Unauthorized` mappe sur 401 (cf. `controller/mod.rs`), pas 403.
//! - `loco_rs::Error::CustomError(StatusCode::FORBIDDEN, ...)` fonctionnerait mais
//!   crée une dépendance sur `ErrorDetail` (struct interne Loco) dans du middleware pur.
//! - Dans un middleware axum, retourner `Ok(response)` est idiomatique : le middleware
//!   court-circuite la chaîne en produisant lui-même la réponse.

use axum::extract::Request;
use axum::http::header::{HOST, ORIGIN, REFERER};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

/// Middleware axum : refuse toute mutation dont le `Origin` (ou `Referer`) ne
/// correspond pas au `Host` de la requête.
///
/// À câbler sur les routes mutantes (POST/PUT/DELETE) via
/// `.layer(axum::middleware::from_fn(require_same_origin))`.
pub async fn require_same_origin(req: Request, next: Next) -> Result<Response, loco_rs::Error> {
    let headers = req.headers();

    let host_hdr = headers
        .get(HOST)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);

    // Récupère le host depuis le header `Host`. En transport HTTP réel (Loco tests),
    // hyper injecte ce header automatiquement. En mode mock sans Host, fallback sur
    // l'authority de l'URI.
    let host = host_hdr.or_else(|| req.uri().authority().map(|a| a.host().to_string()));

    let origin_host = headers
        .get(ORIGIN)
        .or_else(|| headers.get(REFERER))
        .and_then(|v| v.to_str().ok())
        .and_then(url_host);

    match (host, origin_host) {
        (Some(h), Some(o)) if same_host(&h, &o) => Ok(next.run(req).await),
        _ => Ok((StatusCode::FORBIDDEN, "cross-origin mutation refused").into_response()),
    }
}

/// Extrait l'hôte (`host[:port]`) d'une URL `scheme://[userinfo@]host[:port]/...`.
/// Retourne `None` si le format ne correspond pas.
///
/// Le userinfo éventuel (`user@`) est éliminé pour neutraliser les attaques de type
/// `http://victim.com@evil.com/` où l'authority réelle est `evil.com`.
pub(super) fn url_host(raw: &str) -> Option<String> {
    let after_scheme = raw.split("://").nth(1)?;
    // L'authority s'arrête au premier '/'.
    let authority = after_scheme.split('/').next()?;
    // Supprime le userinfo s'il est présent (`user@host` → `host`).
    let host = authority.split('@').next_back()?;
    Some(host.to_string())
}

/// Compare deux `host[:port]` selon la sémantique d'origine HTTP.
///
/// Règles :
/// - Les noms d'hôtes doivent être identiques (comparaison exacte).
/// - Si les deux valeurs incluent un port explicite, ils doivent être égaux.
/// - Si l'une des deux n'a pas de port (ex. `Host: example.com` émis sans port
///   par un proxy ou par le client), on accepte — le port par défaut du schéma
///   ne peut pas être inféré ici sans connaître le protocole.
///
/// Exemples : `"example.com:80"` vs `"example.com"` → `true` ;
/// `"example.com:8080"` vs `"example.com:9090"` → `false`.
pub(super) fn same_host(host_header: &str, origin_host: &str) -> bool {
    let (h_name, h_port) = split_host_port(host_header);
    let (o_name, o_port) = split_host_port(origin_host);

    if h_name != o_name {
        return false;
    }

    // Si les deux ont un port explicite, ils doivent correspondre.
    match (h_port, o_port) {
        (Some(hp), Some(op)) => hp == op,
        // L'un ou les deux n'ont pas de port → on accepte (même hôte suffit).
        _ => true,
    }
}

/// Découpe `"host"` ou `"host:port"` en `(nom, Option<port>)`.
fn split_host_port(hostport: &str) -> (&str, Option<&str>) {
    // Gère IPv6 `[::1]:port` → pour l'instant on traite simplement host:port.
    match hostport.rsplit_once(':') {
        Some((name, port)) => (name, Some(port)),
        None => (hostport, None),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::{same_host, url_host};

    // --- url_host ---

    #[test]
    fn url_host_extrait_hote_simple() {
        assert_eq!(
            url_host("https://example.com/path"),
            Some("example.com".to_string())
        );
    }

    #[test]
    fn url_host_extrait_hote_avec_port() {
        assert_eq!(
            url_host("https://example.com:443/path"),
            Some("example.com:443".to_string())
        );
    }

    #[test]
    fn url_host_sans_chemin() {
        assert_eq!(
            url_host("https://example.com"),
            Some("example.com".to_string())
        );
    }

    #[test]
    fn url_host_url_sans_scheme_retourne_none() {
        assert_eq!(url_host("example.com"), None);
    }

    #[test]
    fn url_host_valeur_vide_retourne_none() {
        assert_eq!(url_host(""), None);
    }

    /// Attaque classique : `http://victim.com@evil.com/` — l'authority réelle est
    /// `evil.com` (le `victim.com` est du userinfo). On doit retourner `evil.com`.
    #[test]
    fn url_host_strips_userinfo() {
        assert_eq!(
            url_host("http://victim.com@evil.com/"),
            Some("evil.com".to_string())
        );
    }

    #[test]
    fn url_host_sans_userinfo_inchange() {
        assert_eq!(
            url_host("https://example.com/path"),
            Some("example.com".to_string())
        );
    }

    // --- same_host ---

    #[test]
    fn same_host_hotes_identiques() {
        assert!(same_host("example.com", "example.com"));
    }

    #[test]
    fn same_host_hotes_avec_ports_identiques() {
        assert!(same_host("example.com:8080", "example.com:8080"));
    }

    #[test]
    fn same_host_host_avec_port_origin_sans_port() {
        // Host: example.com:80 vs Origin: https://example.com/ → même nom d'hôte
        assert!(same_host("example.com:80", "example.com"));
    }

    #[test]
    fn same_host_host_sans_port_origin_avec_port() {
        assert!(same_host("example.com", "example.com:443"));
    }

    #[test]
    fn same_host_hotes_differents_rejete() {
        assert!(!same_host("example.com", "evil.example"));
    }

    #[test]
    fn same_host_sous_domaine_rejete() {
        assert!(!same_host("example.com", "sub.example.com"));
    }

    #[test]
    fn same_host_domaine_parent_rejete() {
        assert!(!same_host("sub.example.com", "example.com"));
    }

    #[test]
    fn same_host_ports_differents_rejete() {
        assert!(!same_host("example.com:8080", "example.com:9090"));
    }

    // --- Tests d'intégration du middleware (tower::ServiceExt::oneshot) ---
    //
    // On construit un Router minimal avec un dummy handler POST et on passe
    // les requêtes directement via oneshot, sans démarrer de serveur TCP.

    use axum::body::Body;
    use axum::http::{Request, StatusCode as HttpStatusCode};
    use axum::middleware;
    use axum::routing::post;
    use axum::Router;
    use tower::ServiceExt;

    /// Construit un `Router` de test avec un POST `/test` protégé par
    /// `require_same_origin`, qui répond 200 si le middleware laisse passer.
    fn test_router() -> Router {
        Router::new().route(
            "/test",
            post(|| async { (HttpStatusCode::OK, "ok") })
                .layer(middleware::from_fn(super::require_same_origin)),
        )
    }

    #[tokio::test]
    async fn middleware_laisse_passer_origin_same_origin() {
        let app = test_router();
        let req = Request::builder()
            .method("POST")
            .uri("/test")
            .header("host", "example.com")
            .header("origin", "https://example.com")
            .body(Body::empty())
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), HttpStatusCode::OK);
    }

    #[tokio::test]
    async fn middleware_rejette_origin_cross_origin() {
        let app = test_router();
        let req = Request::builder()
            .method("POST")
            .uri("/test")
            .header("host", "example.com")
            .header("origin", "https://evil.example")
            .body(Body::empty())
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), HttpStatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn middleware_rejette_sans_origin_ni_referer() {
        // Fail-closed : absence d'Origin ET de Referer → 403.
        let app = test_router();
        let req = Request::builder()
            .method("POST")
            .uri("/test")
            .header("host", "example.com")
            .body(Body::empty())
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), HttpStatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn middleware_laisse_passer_referer_same_origin() {
        // Referer en repli quand Origin est absent.
        let app = test_router();
        let req = Request::builder()
            .method("POST")
            .uri("/test")
            .header("host", "example.com")
            .header("referer", "https://example.com/admin")
            .body(Body::empty())
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), HttpStatusCode::OK);
    }

    #[tokio::test]
    async fn middleware_rejette_referer_cross_origin() {
        let app = test_router();
        let req = Request::builder()
            .method("POST")
            .uri("/test")
            .header("host", "example.com")
            .header("referer", "https://evil.example/page")
            .body(Body::empty())
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), HttpStatusCode::FORBIDDEN);
    }
}
