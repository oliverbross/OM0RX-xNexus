// SPDX-License-Identifier: GPL-3.0-only
use super::*;

#[test]
fn passive_facts_survive_deadlines_but_not_revision_or_write_revocation() {
    let (bridge, mut client) = paired(Readiness::Ready);
    let mut batch = accept(&mut client, wire(Readiness::Ready, NOW)).unwrap();
    batch.expires = Instant::now() - Duration::from_millis(1);
    batch.authority.expires = batch.expires;
    assert!(!batch.is_fresh());
    assert_eq!(batch.remaining_valid_ms(), 0);
    assert!(batch.results().is_err(), "live API must stay expired");
    for n in 1..4 {
        let mut state = client.authority.lock().unwrap();
        state.observe(None, NOW + n).unwrap();
        state
            .observe(Some(&metadata(Readiness::Ready, NOW + n)), NOW + n)
            .unwrap();
        assert_eq!(
            batch.display().unwrap().1[0].need.status,
            crate::NeedStatus::Needed
        );
    }
    let mut changed = metadata(Readiness::Ready, NOW + 4);
    changed.authority.as_mut().unwrap().logbook_revision += 1;
    client
        .authority
        .lock()
        .unwrap()
        .observe(Some(&changed), NOW + 4)
        .unwrap();
    assert!(
        batch.display().is_err(),
        "changed logbook withdraws old facts"
    );
    let mut response = wire(Readiness::Ready, NOW + 4);
    response["authority"]["logbook_revision"] = json!(1);
    let fresh = accept(&mut client, response).unwrap();
    assert!(fresh.display().is_ok());
    bridge.invalidate_needs();
    assert!(
        fresh.display().is_err(),
        "pending writes also withdraw display"
    );
}

fn pending() -> Value {
    json!({"protocol_version":1,"server_instance":"synthetic-server",
        "workspace_id":"synthetic-workspace","error":"needs_pending"})
}
fn unavailable(body: Value) -> String {
    let body = body.to_string();
    format!("HTTP/1.1 503 Service Unavailable\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len())
}

#[test]
fn expired_captured_deadline_does_not_revoke_refreshed_authority() {
    let (_bridge, mut client) = paired(Readiness::Ready);
    let mut old = client.authority.lock().unwrap().lease().unwrap();
    // Model the elapsed A deadline without a sleep-dependent boundary. B is a
    // genuine AuthorityState observation sharing A's still-current identity.
    old.expires = Instant::now() - Duration::from_millis(1);
    client
        .authority
        .lock()
        .unwrap()
        .observe(Some(&metadata(Readiness::Ready, NOW + 1)), NOW + 1)
        .unwrap();
    let fresh = client.authority.lock().unwrap().lease().unwrap();
    assert!(Arc::ptr_eq(&old.live, &fresh.live));
    assert!(matches!(
        client.accept(
            serde_json::from_value(wire(Readiness::Ready, NOW)).unwrap(),
            &queries(),
            old,
            NOW
        ),
        Err(Error::Stale)
    ));
    assert!(fresh.fresh(), "late A must not revoke B's fresh authority");
    assert!(client.remaining_authority_ms() > 0);
    assert!(accept(&mut client, wire(Readiness::Ready, NOW + 1))
        .unwrap()
        .is_fresh());
}

#[test]
fn authenticated_pending_preserves_unrelated_batch_without_renewal() {
    let now = now_ms().unwrap();
    let (bridge, mut client, server) = network_client(vec![
        crate::tests::json_response(wire(Readiness::Ready, now)),
        unavailable(pending()),
    ]);
    let a = client.query(&queries()).unwrap();
    let deadline = a.expires.min(a.authority.expires);
    let before = a.remaining_valid_ms();
    let mut b = queries();
    b[0].callsign = "W9XYZ".into();
    b[0].query_id = "group_b".into();
    assert!(
        matches!(client.query(&b), Err(Error::NeedsPending)),
        "pending is never a fabricated result"
    );
    assert!(a.is_fresh(), "pending B must not erase valid A");
    assert_eq!(a.expires.min(a.authority.expires), deadline);
    assert!(a.remaining_valid_ms() <= before);
    assert!(bridge.snapshot().is_ok());
    assert_eq!(server.join().unwrap().len(), 4);
}

#[test]
fn ordinary_or_malformed_503_still_revokes_previous_facts() {
    for mutation in 0..8 {
        let mut value = pending();
        match mutation {
            0 => value = json!({"error":"Needs lookup pending."}),
            1 => value["error"] = json!("needs_busy"),
            2 => value["unexpected"] = json!(true),
            3 => {
                value.as_object_mut().unwrap().remove("workspace_id");
            }
            4 => value["server_instance"] = json!("foreign-server"),
            5 => value["workspace_id"] = json!("foreign-workspace"),
            6 => value["protocol_version"] = json!(2),
            _ => value["error"] = json!(false),
        }
        let now = now_ms().unwrap();
        let (_bridge, mut client, server) = network_client(vec![
            crate::tests::json_response(wire(Readiness::Ready, now)),
            unavailable(value),
        ]);
        let a = client.query(&queries()).unwrap();
        assert!(client.query(&queries()).is_err());
        assert!(!a.is_fresh(), "mutation {mutation}");
        assert_eq!(server.join().unwrap().len(), 4);
    }
}

#[test]
fn pending_parser_keeps_bound_content_type_and_original_lease_fences() {
    for case in 0..5 {
        let body = unavailable(pending());
        let reply = match case {
            0 => body.replace("application/json", "text/plain"),
            1 => unavailable(json!({"error":"x".repeat(1025)})),
            _ => body,
        };
        let (bridge, client, server) = network_client(vec![reply]);
        let mut lease = client.authority.lock().unwrap().lease().unwrap();
        if case == 2 {
            lease.expires = Instant::now() - Duration::from_millis(1);
        }
        if case == 3 {
            bridge.invalidate_needs();
        }
        if case == 4 {
            client.session.store(false, Ordering::Release);
        }
        let response = client
            .http
            .post(format!("{}/xlog/v1/needs/query", client.endpoint))
            .timeout(Duration::from_secs(1))
            .body("{}")
            .send()
            .unwrap();
        let error = client.pending_error(response, &lease);
        assert_eq!(
            error,
            match case {
                0 => Error::Http(503),
                1 => Error::TooLarge,
                4 => Error::Disconnected,
                _ => Error::Stale,
            }
        );
        if case == 2 {
            assert!(client.remaining_authority_ms() > 0);
        }
        assert_eq!(server.join().unwrap().len(), 3);
    }
}

#[test]
fn real_revision_change_and_write_revocation_still_retire_old_authority() {
    for changed_revision in [false, true] {
        let (bridge, mut client) = paired(Readiness::Ready);
        let a = accept(&mut client, wire(Readiness::Ready, NOW)).unwrap();
        if changed_revision {
            let mut next = metadata(Readiness::Ready, NOW + 1);
            next.authority.as_mut().unwrap().logbook_revision += 1;
            client
                .authority
                .lock()
                .unwrap()
                .observe(Some(&next), NOW + 1)
                .unwrap();
        } else {
            bridge.invalidate_needs();
        }
        assert!(!a.is_fresh());
        assert!(a.results().is_err());
    }
    let (_bridge, mut client) = paired(Readiness::Ready);
    let a = accept(&mut client, wire(Readiness::Ready, NOW)).unwrap();
    let mut changed = wire(Readiness::Ready, NOW + 1);
    changed["authority"]["logbook_revision"] = json!(1);
    assert!(matches!(accept(&mut client, changed), Err(Error::Stale)));
    assert!(
        !a.is_fresh(),
        "an actually changed reply authority still revokes A"
    );
    assert_eq!(client.remaining_authority_ms(), 0);
}

#[test]
fn transient_transport_preserves_prior_facts_but_not_beyond_original_deadline() {
    let (bridge, mut client) = paired(Readiness::Ready);
    let mut batch = accept(&mut client, wire(Readiness::Ready, NOW)).unwrap();
    let deadline = batch.expires;
    // The paired fixture's loopback port has no listener: an actual failed read.
    assert!(matches!(client.query(&queries()), Err(Error::Transport)));
    assert!(batch.is_fresh());
    assert_eq!(batch.expires, deadline);
    batch.expires = Instant::now() - Duration::from_millis(1);
    assert!(!batch.is_fresh());
    bridge.invalidate_needs();
    assert!(!batch.authority.live.load(Ordering::Acquire));
}

#[test]
fn stale_refresh_response_does_not_erase_valid_earlier_result() {
    let now = now_ms().unwrap();
    let (bridge, mut client, server) = network_client(vec![
        crate::tests::json_response(wire(Readiness::Ready, now)),
        crate::tests::json_response(wire(Readiness::Ready, now - 2001)),
    ]);
    let batch = client.query(&queries()).unwrap();
    let deadline = batch.expires.min(batch.authority.expires);
    assert!(matches!(client.query(&queries()), Err(Error::Stale)));
    assert!(batch.is_fresh());
    assert_eq!(batch.expires.min(batch.authority.expires), deadline);
    bridge.invalidate_needs();
    assert!(!batch.is_fresh());
    assert_eq!(server.join().unwrap().len(), 4);
}
