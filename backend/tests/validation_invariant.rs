#![allow(clippy::unwrap_used)]
//! Invariant §9 : chaque DTO de frontière rejette une entrée hors-borne. Type-level
//! garanti par ValidatedJson<T: Validate> + args.validate() (compile). Ici : couverture
//! comportementale table-driven — une régression de borne casse le build.

use latch::dto::*;
use validator::Validate;

#[test]
fn every_write_dto_rejects_oversized_input() {
    // (label, closure renvoyant un DTO hors-borne) → doit être Err(validate)
    macro_rules! reject {
        ($label:expr, $dto:expr) => {
            assert!($dto.validate().is_err(), "{} devrait être rejeté", $label);
        };
    }
    reject!(
        "CreateProjectReq.name vide",
        CreateProjectReq {
            name: "".into(),
            brand_name: None,
            code_enabled: false,
            pin: None,
            comments_enabled: None
        }
    );
    reject!(
        "CreateProjectReq.name >128",
        CreateProjectReq {
            name: "x".repeat(129),
            brand_name: None,
            code_enabled: false,
            pin: None,
            comments_enabled: None
        }
    );
    reject!(
        "CreateProjectReq.brand_name >128",
        CreateProjectReq {
            name: "ok".into(),
            brand_name: Some("x".repeat(129)),
            code_enabled: false,
            pin: None,
            comments_enabled: None
        }
    );
    reject!("SetCodeReq.pin", SetCodeReq { pin: "42".into() });
    reject!(
        "DeployReq.html vide",
        DeployReq {
            html: "".into(),
            activate: false,
            notes: None
        }
    );
    reject!(
        "DeployReq.notes >10000",
        DeployReq {
            html: "<h1>x</h1>".into(),
            activate: false,
            notes: Some("x".repeat(10_001))
        }
    );
    reject!(
        "LoginReq.user vide",
        LoginReq {
            user: "".into(),
            pass: "x".into()
        }
    );
    reject!(
        "CreatePinReq.body >2000",
        CreatePinReq {
            anchor: "{}".into(),
            author_name: "A".into(),
            body: "x".repeat(2001)
        }
    );
    reject!(
        "CreatePinReq.author >80",
        CreatePinReq {
            anchor: "{}".into(),
            author_name: "x".repeat(81),
            body: "hi".into()
        }
    );
    reject!(
        "ReplyReq.body >2000",
        ReplyReq {
            author_name: "A".into(),
            body: "x".repeat(2001)
        }
    );
    reject!(
        "EditMessageReq.body vide",
        EditMessageReq { body: "".into() }
    );
    reject!(
        "AdminCreatePinReq.body >2000",
        AdminCreatePinReq {
            anchor: "{}".into(),
            body: "x".repeat(2001)
        }
    );
    reject!("AdminReplyReq.body vide", AdminReplyReq { body: "".into() });
    reject!("UnlockReq.pin", UnlockReq { pin: "abc".into() });
}
