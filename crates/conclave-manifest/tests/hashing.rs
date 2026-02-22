use conclave_manifest::*;
use std::collections::BTreeMap;

fn minimal_manifest() -> Manifest {
    Manifest {
        conclave_manifest_version: "0.1".into(),
        program: Program {
            name: "test".into(),
            plan_ir_hash: "sha256:0000000000000000000000000000000000000000000000000000000000000000"
                .into(),
        },
        target: Target {
            triple: "aarch64-apple-darwin".into(),
            os: "macos".into(),
            arch: "aarch64".into(),
        },
        toolchain: Toolchain {
            lowerer_hash: "sha256:1111111111111111111111111111111111111111111111111111111111111111"
                .into(),
            runtime_hash: "sha256:2222222222222222222222222222222222222222222222222222222222222222"
                .into(),
            stdlib_hash: "sha256:3333333333333333333333333333333333333333333333333333333333333333"
                .into(),
        },
        capability_bindings: BTreeMap::new(),
        scheduler_policy: SchedulerPolicy {
            strategy: "bounded_parallel_map".into(),
            max_inflight: 2,
            ready_queue_order: vec![],
            node_kind_order: vec![],
            tie_breaker: TieBreaker {
                kind: "stable".into(),
                seed: 0,
            },
        },
        determinism: Determinism {
            mode: "sealed_replay".into(),
            clock: "virtual".into(),
            randomness: RandomnessPolicy {
                allowed: false,
                seed: 0,
                source: "none".into(),
            },
            float: "strict".into(),
            io_policy: IoPolicy {
                network: NetworkPolicy::Deny,
                filesystem: FilesystemPolicy::Deny,
                env: EnvPolicy::Frozen,
            },
        },
        observability: Observability {
            trace_level: "deterministic".into(),
            emit_scheduler_trace: true,
            emit_capability_metrics: false,
        },
        supply_chain: SupplyChain {
            artifact_store: "content_addressed".into(),
            require_artifact_signatures: false,
            manifest_signature: None,
        },
    }
}

#[test]
fn canonical_manifest_hash_is_deterministic() {
    let m = minimal_manifest();
    let h1 = compute_canonical_manifest_hash(&m);
    let h2 = compute_canonical_manifest_hash(&m);
    assert_eq!(h1, h2);
}

#[test]
fn canonical_manifest_hash_has_sha256_prefix() {
    let m = minimal_manifest();
    let h = compute_canonical_manifest_hash(&m);
    assert!(h.to_string().starts_with("sha256:"));
}

#[test]
fn canonical_manifest_hash_differs_with_different_program_name() {
    let mut m1 = minimal_manifest();
    let mut m2 = minimal_manifest();
    m1.program.name = "prog_a".into();
    m2.program.name = "prog_b".into();
    assert_ne!(
        compute_canonical_manifest_hash(&m1),
        compute_canonical_manifest_hash(&m2)
    );
}

#[test]
fn canonical_manifest_hash_excludes_signature_field() {
    // Two manifests with the same manifest_signature metadata (algo, public_key_id)
    // but DIFFERENT signature values must hash identically — only the signature bytes
    // are excluded from the hash (a signature cannot sign itself).
    let sig_a = ManifestSignature {
        algo: "ed25519".into(),
        public_key_id: "kid:test".into(),
        signature: "base64:abc123".into(),
    };
    let sig_b = ManifestSignature {
        algo: "ed25519".into(),
        public_key_id: "kid:test".into(),
        signature: "base64:totally_different_value".into(),
    };

    let mut m_a = minimal_manifest();
    let mut m_b = minimal_manifest();
    m_a.supply_chain.manifest_signature = Some(sig_a);
    m_b.supply_chain.manifest_signature = Some(sig_b);

    assert_eq!(
        compute_canonical_manifest_hash(&m_a),
        compute_canonical_manifest_hash(&m_b),
        "manifest hash must not include the signature value"
    );

    // But a different public_key_id DOES affect the hash (only the signature value is excluded).
    let mut m_c = minimal_manifest();
    m_c.supply_chain.manifest_signature = Some(ManifestSignature {
        algo: "ed25519".into(),
        public_key_id: "kid:other".into(),
        signature: "base64:abc123".into(),
    });
    assert_ne!(
        compute_canonical_manifest_hash(&m_a),
        compute_canonical_manifest_hash(&m_c),
        "manifest hash must include public_key_id"
    );
}
