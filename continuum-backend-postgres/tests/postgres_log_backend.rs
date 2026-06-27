//! `PostgreSQL` contract tests.
//!
//! Requires `CONTINUUM_TEST_POSTGRES_URL`. Run with:
//! ```text
//! CONTINUUM_TEST_POSTGRES_URL=postgres://... cargo test -p continuum-backend-postgres -- --ignored
//! ```

use continuum_backend_postgres::PostgresLogBackend;
use continuum_core::backend::LogBackend;
use continuum_core::types::{Seq, SubscriptionId};
use continuum_test_utils::contract;
use continuum_test_utils::fixtures::{sample_record, BackendEnv};
use sqlx::PgPool;
use uuid::Uuid;

const ENV: BackendEnv = BackendEnv::POSTGRES;

fn postgres_url() -> Option<String> {
    std::env::var("CONTINUUM_TEST_POSTGRES_URL").ok()
}

/// Isolated database for one test (mirrors `SQLite` temp-file isolation).
struct TestDb {
    admin: PgPool,
    url: String,
    name: String,
}

impl TestDb {
    async fn new(base_url: &str) -> Self {
        let name = format!("ct_{}", Uuid::new_v4().as_simple());
        let admin = PgPool::connect(base_url)
            .await
            .expect("connect to admin database");
        sqlx::query(&format!("CREATE DATABASE {name}"))
            .execute(&admin)
            .await
            .expect("create test database");
        let (prefix, _) = base_url
            .rsplit_once('/')
            .expect("CONTINUUM_TEST_POSTGRES_URL must include database name");
        let url = format!("{prefix}/{name}");
        Self { admin, url, name }
    }

    async fn drop(self) {
        let terminate = format!(
            "SELECT pg_terminate_backend(pid) FROM pg_stat_activity \
             WHERE datname = '{}' AND pid <> pg_backend_pid()",
            self.name
        );
        let _ = sqlx::query(&terminate).execute(&self.admin).await;
        let drop_db = format!("DROP DATABASE IF EXISTS {}", self.name);
        let _ = sqlx::query(&drop_db).execute(&self.admin).await;
    }
}

async fn with_backend<F, Fut>(f: F)
where
    F: FnOnce(PostgresLogBackend) -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    let base = postgres_url().expect("CONTINUUM_TEST_POSTGRES_URL not set");
    let db = TestDb::new(&base).await;
    let backend = PostgresLogBackend::new(&db.url)
        .await
        .expect("backend");
    f(backend).await;
    db.drop().await;
}

async fn with_db<F, Fut>(f: F)
where
    F: FnOnce(String) -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    let base = postgres_url().expect("CONTINUUM_TEST_POSTGRES_URL not set");
    let db = TestDb::new(&base).await;
    f(db.url.clone()).await;
    db.drop().await;
}

macro_rules! postgres_test {
    ($name:ident, $body:expr) => {
        #[tokio::test]
        #[ignore = "requires CONTINUUM_TEST_POSTGRES_URL"]
        async fn $name() {
            if postgres_url().is_none() {
                eprintln!("skip: CONTINUUM_TEST_POSTGRES_URL not set");
                return;
            }
            $body.await;
        }
    };
}

postgres_test!(p1_create_on_append, async {
    with_backend(|b| async move {
        contract::append_single(&b, &ENV).await;
    })
    .await;
});

postgres_test!(p2_durable_after_reopen, async {
    with_db(|url| async move {
        let stream = ENV.stream("t");
        {
            let b = PostgresLogBackend::new(&url).await.unwrap();
            b.append(stream.clone(), &[sample_record()]).await.unwrap();
        }
        let b2 = PostgresLogBackend::new(&url).await.unwrap();
        let rows = b2.read_from(stream, Seq::ZERO, 10).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].seq, Seq(1));
    })
    .await;
});

postgres_test!(p3_batch_100, async {
    with_backend(|b| async move {
        let stream = ENV.stream("t");
        let recs: Vec<_> = (0..100).map(|_| sample_record()).collect();
        let seqs = b.append(stream, &recs).await.unwrap();
        assert_eq!(seqs.len(), 100);
        assert_eq!(seqs[0], Seq(1));
        assert_eq!(seqs[99], Seq(100));
    })
    .await;
});

postgres_test!(p4_duplicate_event_id, async {
    with_backend(|b| async move {
        contract::duplicate_event_id(&b, &ENV).await;
    })
    .await;
});

postgres_test!(p5_read_semantics, async {
    with_backend(|b| async move {
        contract::read_from_start(&b, &ENV).await;
        contract::read_from_mid(&b, &ENV).await;
        contract::read_limit_zero(&b, &ENV).await;
        contract::read_wrong_stream(&b, &ENV).await;
    })
    .await;
});

postgres_test!(p6_checkpoint_reopen, async {
    with_db(|url| async move {
        let stream = ENV.stream("t");
        let sub = SubscriptionId::new("sub");
        {
            let b = PostgresLogBackend::new(&url).await.unwrap();
            let seqs = b.append(stream.clone(), &[sample_record()]).await.unwrap();
            b.commit_checkpoint(&sub, stream.clone(), seqs[0])
                .await
                .unwrap();
        }
        let b2 = PostgresLogBackend::new(&url).await.unwrap();
        assert_eq!(
            b2.load_checkpoint(&sub, stream).await.unwrap(),
            Some(Seq(1))
        );
    })
    .await;
});

postgres_test!(p7_truncate_logical, async {
    with_backend(|b| async move {
        contract::truncate(&b, &ENV).await;
    })
    .await;
});

postgres_test!(p8_schema_idempotent, async {
    with_db(|url| async move {
        PostgresLogBackend::new(&url).await.unwrap();
        PostgresLogBackend::new(&url).await.unwrap();
    })
    .await;
});

postgres_test!(p9_partition_keys_independent, async {
    with_backend(|b| async move {
        contract::distinct_partition_keys(&b, &ENV).await;
    })
    .await;
});

postgres_test!(p10_independent_destinations, async {
    with_backend(|b| async move {
        contract::independent_destinations(&b, &ENV).await;
    })
    .await;
});

postgres_test!(read_from_topic_all_keys, async {
    with_backend(|b| async move {
        contract::read_from_topic_all_keys(&b, &ENV).await;
    })
    .await;
});

postgres_test!(read_from_topic_single_key, async {
    with_backend(|b| async move {
        contract::read_from_topic_single_key(&b, &ENV).await;
    })
    .await;
});

postgres_test!(read_from_topic_after_and_limit, async {
    with_backend(|b| async move {
        contract::read_from_topic_after_and_limit(&b, &ENV).await;
    })
    .await;
});

postgres_test!(empty_topic_rejected, async {
    with_backend(|b| async move {
        contract::empty_topic_rejected(&b, &ENV).await;
    })
    .await;
});

postgres_test!(read_limit_validation, async {
    with_backend(|b| async move {
        contract::read_limit_validation(&b, &ENV).await;
    })
    .await;
});

postgres_test!(append_batch, async {
    with_backend(|b| async move {
        contract::append_batch(&b, &ENV).await;
    })
    .await;
});

postgres_test!(append_empty, async {
    with_backend(|b| async move {
        contract::append_empty(&b, &ENV).await;
    })
    .await;
});

postgres_test!(checkpoint_none, async {
    with_backend(|b| async move {
        contract::checkpoint_none(&b, &ENV).await;
    })
    .await;
});

postgres_test!(checkpoint_roundtrip, async {
    with_backend(|b| async move {
        contract::checkpoint_roundtrip(&b, &ENV).await;
    })
    .await;
});

postgres_test!(checkpoint_monotonic, async {
    with_backend(|b| async move {
        contract::checkpoint_monotonic(&b, &ENV).await;
    })
    .await;
});

postgres_test!(truncate_before_min, async {
    with_backend(|b| async move {
        contract::truncate_before_min(&b, &ENV).await;
    })
    .await;
});

postgres_test!(e2e_lifecycle, async {
    with_backend(|b| async move {
        contract::e2e_lifecycle(&b, &ENV).await;
    })
    .await;
});
