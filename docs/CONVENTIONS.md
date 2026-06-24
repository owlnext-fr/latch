# Conventions — squelettes de code du projet

> Les patterns *découverts en cours de route* (un service type, un endpoint type, un
> composant Yew type, un test type). À remplir au fil de l'implémentation : dès qu'un
> motif se répète, on le fige ici pour que les sessions suivantes le copient au lieu
> de le réinventer. Les règles *normatives fixées d'avance* (pas d'`unwrap`, commits
> conventionnels…) restent dans `BOOTSTRAP §4`, pas ici.

## Service (cœur) type
_(à remplir : signature d'un service avec ses dépendances injectées —
`DeployService { db, storage: Arc<dyn Storage> }` — et un exemple de méthode rendant
un `Result<_, CoreError>`.)_

## Endpoint admin (adaptateur web) type
_(à remplir : un handler JSON qui extrait, appelle un service, mappe `CoreError` →
status + JSON, avec la vérif `Origin` sur mutation.)_

## Tool MCP type
_(à remplir : un tool qui valide `deploy_token` en premier, puis appelle le service,
puis mappe l'erreur en tool error.)_

## Composant Yew (shadcn-rs) type
_(à remplir : un écran admin type, side-panel + appel API JSON.)_

## Test d'intégration type
_(à remplir : montage Loco + SQLite de test, assertion, + le pattern du test-invariant
de sécu « aucun hash / aucun PIN en liste ».)_
