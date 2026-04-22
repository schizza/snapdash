 use super::*;

 // ================================================================
 // ws_url_from_http — HTTP(s) → WS(s) URL rewriting
 // ================================================================
 mod url_conversion {
     use super::*;

     #[test]
     fn https_becomes_wss() {
         let u = ws_url_from_http("https://ha.local/").unwrap();
         assert_eq!(u.as_str(), "wss://ha.local/api/websocket");
     }

     #[test]
     fn http_becomes_ws() {
         let u = ws_url_from_http("http://ha.local/").unwrap();
         assert_eq!(u.as_str(), "ws://ha.local/api/websocket");
     }

     #[test]
     fn preserves_port() {
         let u = ws_url_from_http("http://ha.local:8123/").unwrap();
         assert_eq!(u.as_str(), "ws://ha.local:8123/api/websocket");
     }

     #[test]
     fn preserves_ipv4_host() {
         let u = ws_url_from_http("http://192.168.1.10:8123/").unwrap();
         assert_eq!(u.as_str(), "ws://192.168.1.10:8123/api/websocket");
     }

     #[test]
     fn preserves_ipv6_host() {
         let u = ws_url_from_http("http://[::1]:8123/").unwrap();
         assert_eq!(u.as_str(), "ws://[::1]:8123/api/websocket");
     }

     #[test]
     fn preserves_subpath_with_trailing_slash() {
         // HA za reverse proxy na /ha/
         let u = ws_url_from_http("https://ha.local/ha/").unwrap();
         assert_eq!(u.as_str(), "wss://ha.local/ha/api/websocket");
     }

     #[test]
     fn preserves_subpath_without_trailing_slash() {
         let u = ws_url_from_http("https://ha.local/ha").unwrap();
         assert_eq!(u.as_str(), "wss://ha.local/ha/api/websocket");
     }

     #[test]
     fn preserves_deep_subpath() {
         let u = ws_url_from_http("https://proxy.example.com/services/ha/").unwrap();
         assert_eq!(
             u.as_str(),
             "wss://proxy.example.com/services/ha/api/websocket"
         );
     }

     #[test]
     fn no_path_component_works() {
         let u = ws_url_from_http("https://ha.local").unwrap();
         assert_eq!(u.as_str(), "wss://ha.local/api/websocket");
     }

     #[test]
     fn lowercases_scheme() {
         // RFC 3986: schémata case-insensitive; url crate normalizuje
         let u = ws_url_from_http("HTTPS://ha.local/").unwrap();
         assert_eq!(u.as_str(), "wss://ha.local/api/websocket");
     }

     #[test]
     fn collapses_redundant_trailing_slashes() {
         // guard proti "//api/websocket" bugu
         let u = ws_url_from_http("https://ha.local///").unwrap();
         assert!(!u.path().contains("//"));
         assert!(u.path().ends_with("/api/websocket"));
     }

     #[test]
     fn preserves_userinfo() {
         let u = ws_url_from_http("https://user:pass@ha.local/").unwrap();
         assert!(u.as_str().contains("user:pass@"));
         assert!(u.as_str().ends_with("/api/websocket"));
     }

     #[test]
     fn preserves_query_string() {
         let u = ws_url_from_http("https://ha.local/?foo=bar").unwrap();
         assert_eq!(u.query(), Some("foo=bar"));
     }

     #[test]
     fn rejects_ftp_scheme() {
         let err = ws_url_from_http("ftp://ha.local/").unwrap_err();
         assert!(matches!(err, HaError::Protocol(_)));
     }

     #[test]
     fn rejects_preconverted_ws_scheme() {
         // API kontrakt: vstup je http(s), ne ws(s)
         let err = ws_url_from_http("wss://ha.local/").unwrap_err();
         assert!(matches!(err, HaError::Protocol(_)));
     }

     #[test]
     fn rejects_missing_scheme() {
         let err = ws_url_from_http("ha.local:8123").unwrap_err();
         assert!(matches!(err, HaError::Protocol(_)));
     }

     #[test]
     fn rejects_empty_string() {
         assert!(matches!(
             ws_url_from_http("").unwrap_err(),
             HaError::Protocol(_)
         ));
     }

     #[test]
     fn rejects_garbage_input() {
         assert!(matches!(
             ws_url_from_http("not a url at all").unwrap_err(),
             HaError::Protocol(_)
         ));
     }

     #[test]
     fn error_message_is_descriptive() {
         // UX detail: uživatel uvidí v UI status baru → stojí za to
         let err = ws_url_from_http("ftp://ha/").unwrap_err();
         let HaError::Protocol(msg) = err else {
             panic!()
         };
         assert!(
             msg.contains("scheme"),
             "expected 'scheme' in error message, got: {msg}"
         );
     }

     #[test]
     fn collapses_leading_double_slash_in_path() {
         // uživatel zkopíroval URL z dokumentace kde bylo //
         let u = ws_url_from_http("https://ha.local//ha/").unwrap();
         assert_eq!(u.as_str(), "wss://ha.local/ha/api/websocket");
     }

     #[test]
     fn handles_only_path_with_slashes() {
         let u = ws_url_from_http("https://ha.local////").unwrap();
         assert_eq!(u.as_str(), "wss://ha.local/api/websocket");
     }

     #[test]
     fn preserves_middle_double_slash_if_user_really_put_it_there() {
         // middle // je v URL spec validní; my ho nemažeme (je to možná záměr)
         let u = ws_url_from_http("https://ha.local/a//b/").unwrap();
         assert_eq!(u.as_str(), "wss://ha.local/a//b/api/websocket");
     }
 }

 // ================================================================
 // ServerMsg — deserializace příchozích zpráv z HA
 // ================================================================
 mod server_msg_decode {
     use super::*;

     #[test]
     fn auth_required_with_version() {
         let msg: ServerMsg =
             serde_json::from_str(r#"{"type":"auth_required","ha_version":"2024.5.0"}"#)
                 .unwrap();
         assert!(matches!(msg, ServerMsg::AuthRequired { .. }));
     }

     #[test]
     fn auth_required_without_version() {
         let msg: ServerMsg = serde_json::from_str(r#"{"type":"auth_required"}"#).unwrap();
         assert!(matches!(msg, ServerMsg::AuthRequired { .. }));
     }

     #[test]
     fn auth_ok_decodes() {
         let msg: ServerMsg =
             serde_json::from_str(r#"{"type":"auth_ok","ha_version":"2024.5"}"#).unwrap();
         assert!(matches!(msg, ServerMsg::AuthOk { .. }));
     }

     #[test]
     fn auth_invalid_carries_message() {
         let msg: ServerMsg =
             serde_json::from_str(r#"{"type":"auth_invalid","message":"Invalid access token"}"#)
                 .unwrap();
         let ServerMsg::AuthInvalid { message } = msg else {
             panic!("expected AuthInvalid, got {msg:?}");
         };
         assert_eq!(message, "Invalid access token");
     }

     #[test]
     fn auth_invalid_defaults_empty_message() {
         let msg: ServerMsg = serde_json::from_str(r#"{"type":"auth_invalid"}"#).unwrap();
         let ServerMsg::AuthInvalid { message } = msg else {
             panic!()
         };
         assert!(message.is_empty());
     }

     #[test]
     fn state_changed_event_full_payload() {
         let raw = r#"{
             "type":"event",
             "id":1,
             "event":{
                 "event_type":"state_changed",
                 "data":{
                     "new_state":{
                         "entity_id":"light.kitchen",
                         "state":"on",
                         "attributes":{"brightness":255}
                     }
                 }
             }
         }"#;
         let msg: ServerMsg = serde_json::from_str(raw).unwrap();
         let ServerMsg::Event { event, .. } = msg else {
             panic!("expected Event, got {msg:?}")
         };
         assert_eq!(event.event_type, "state_changed");
         let new_state = event.data.new_state.expect("new_state present");
         assert_eq!(new_state.entity_id, "light.kitchen");
         assert_eq!(new_state.state, "on");
     }

     #[test]
     fn event_without_new_state_is_ok() {
         // typicky u service_called, call_service ack, atd.
         let raw = r#"{"type":"event","event":{"event_type":"service_called","data":{}}}"#;
         let msg: ServerMsg = serde_json::from_str(raw).unwrap();
         let ServerMsg::Event { event, .. } = msg else {
             panic!()
         };
         assert!(event.data.new_state.is_none());
     }

     #[test]
     fn event_without_data_field_is_ok() {
         let raw = r#"{"type":"event","event":{"event_type":"whatever"}}"#;
         let msg: ServerMsg = serde_json::from_str(raw).unwrap();
         assert!(matches!(msg, ServerMsg::Event { .. }));
     }

     #[test]
     fn event_without_id_is_ok() {
         // některé HA eventy chodí bez id
         let raw = r#"{"type":"event","event":{"event_type":"state_changed","data":{}}}"#;
         let msg: ServerMsg = serde_json::from_str(raw).unwrap();
         assert!(matches!(msg, ServerMsg::Event { id: None, .. }));
     }

     #[test]
     fn result_success_decodes() {
         let msg: ServerMsg =
             serde_json::from_str(r#"{"type":"result","id":1,"success":true}"#).unwrap();
         assert!(matches!(msg, ServerMsg::Result { success: true, .. }));
     }

     #[test]
     fn result_with_error_payload() {
         let raw = r#"{"type":"result","id":2,"success":false,"error":{"code":"not_found"}}"#;
         let msg: ServerMsg = serde_json::from_str(raw).unwrap();
         assert!(matches!(
             msg,
             ServerMsg::Result {
                 success: false,
                 error: Some(_),
                 ..
             }
         ));
     }

     #[test]
     fn pong_decodes() {
         let msg: ServerMsg = serde_json::from_str(r#"{"type":"pong","id":5}"#).unwrap();
         assert!(matches!(msg, ServerMsg::Pong { .. }));
     }

     #[test]
     fn unknown_type_falls_to_other() {
         // forward-compat: HA někdy přidá nový type — klient nesmí spadnout
         let msg: ServerMsg =
             serde_json::from_str(r#"{"type":"future_thing","whatever":42}"#).unwrap();
         assert!(matches!(msg, ServerMsg::Other));
     }

     #[test]
     fn extra_fields_are_ignored() {
         // HA přidá nové pole do existujícího type → zpracujeme dál
         let raw = r#"{"type":"auth_ok","ha_version":"2024","__new_field__":"x"}"#;
         assert!(matches!(
             serde_json::from_str::<ServerMsg>(raw).unwrap(),
             ServerMsg::AuthOk { .. }
         ));
     }

     #[test]
     fn missing_type_is_error() {
         // bez type tagu serde neví, kterou variantu vybrat
         assert!(serde_json::from_str::<ServerMsg>(r#"{"some":"thing"}"#).is_err());
     }

     #[test]
     fn malformed_json_is_error() {
         assert!(serde_json::from_str::<ServerMsg>("not json").is_err());
     }

     #[test]
     fn empty_object_is_error() {
         assert!(serde_json::from_str::<ServerMsg>("{}").is_err());
     }
 }

 // ================================================================
 // ClientMsg — serializace odchozích zpráv na HA
 // ================================================================
 mod client_msg_encode {
     use super::*;

     #[test]
     fn auth_msg_has_correct_wire_format() {
         let msg = ClientMsg::Auth {
             access_token: "secret",
         };
         let v: serde_json::Value =
             serde_json::from_str(&serde_json::to_string(&msg).unwrap()).unwrap();
         assert_eq!(v["type"], "auth");
         assert_eq!(v["access_token"], "secret");
     }

     #[test]
     fn subscribe_events_has_correct_wire_format() {
         let msg = ClientMsg::SubscribeEvents {
             id: 1,
             event_type: "state_changed",
         };
         let v: serde_json::Value =
             serde_json::from_str(&serde_json::to_string(&msg).unwrap()).unwrap();
         assert_eq!(v["type"], "subscribe_events");
         assert_eq!(v["id"], 1);
         assert_eq!(v["event_type"], "state_changed");
     }

     #[test]
     fn token_with_special_chars_survives_roundtrip() {
         let msg = ClientMsg::Auth {
             access_token: "ab\"c\ne\\f",
         };
         let json = serde_json::to_string(&msg).unwrap();
         let v: serde_json::Value = serde_json::from_str(&json).unwrap();
         assert_eq!(v["access_token"], "ab\"c\ne\\f");
     }

     #[test]
     fn unicode_in_token_survives_roundtrip() {
         let msg = ClientMsg::Auth {
             access_token: "č🦀ü",
         };
         let json = serde_json::to_string(&msg).unwrap();
         let v: serde_json::Value = serde_json::from_str(&json).unwrap();
         assert_eq!(v["access_token"], "č🦀ü");
     }
 }

 // ================================================================
 // Sanity checks pro konstanty — zachytí bugy typu "překlep v čísle"
 // ================================================================
 mod constants {
     use super::*;

     #[test]
     fn backoff_bounds_are_sane() {
         assert!(INITIAL_BACKOFF <= MAX_BACKOFF);
         assert!(INITIAL_BACKOFF > Duration::ZERO);
     }

     #[test]
     fn heartbeat_fires_before_stale_threshold() {
         // musíme pingnout aspoň jednou před tím, než bychom prohlásili stale
         // jinak by se spojení ukončilo dřív, než by vůbec mělo šanci
         assert!(HEARTBEAT_INTERVAL < STALE_AFTER);
     }

     #[test]
     fn auth_failures_bound_is_positive() {
         // 0 by znamenalo "okamžitě vzdej", nemá smysl
         assert!(MAX_AUTH_FAILURES > 0);
     }

     #[test]
     fn event_channel_has_capacity() {
         assert!(EVENT_CHANNEL_CAPACITY > 0);
     }
 }

