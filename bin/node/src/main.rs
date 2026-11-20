#![doc = include_str!("../README.md")]
#![doc(issue_tracker_base_url = "https://github.com/base/base/issues/")]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]

use base_execution_cli::{Cli, StandardBaseRethNode, StandardNodeArgs};

type NodeCli = Cli<StandardNodeArgs>;

#[global_allocator]
static ALLOC: reth_cli_util::allocator::Allocator = reth_cli_util::allocator::new_allocator();

#[cfg(all(feature = "jemalloc-prof", unix))]
#[unsafe(export_name = "malloc_conf")]
static MALLOC_CONF: &[u8] = b"prof:true,prof_active:true,lg_prof_sample:19\0";

/// With `RETH_STATE_STORE_QMDB=<dir>`, keeps current state and its commitment in the QMDB
/// database at `<dir>` (created by `qmdb-migrate`) instead of the hashed state and trie tables.
/// `RETH_STATE_STORE_QMDB_CACHE_PAGES` (4 KiB pages), `RETH_STATE_STORE_QMDB_THREADS` and
/// `RETH_STATE_STORE_QMDB_RETAIN_BLOCKS` (history kept before pruning) tune it.
fn install_state_store() {
    let Some(dir) = std::env::var_os("RETH_STATE_STORE_QMDB") else { return };
    let mut config = reth_state_store_qmdb::QmdbConfig::new(dir);
    let env_usize = |name| std::env::var(name).ok().and_then(|v| v.parse().ok());
    if let Some(pages) = env_usize("RETH_STATE_STORE_QMDB_CACHE_PAGES") {
        config.page_cache_pages = pages;
    }
    if let Some(threads) = env_usize("RETH_STATE_STORE_QMDB_THREADS") {
        config.threads = threads;
    }
    if let Some(blocks) =
        std::env::var("RETH_STATE_STORE_QMDB_RETAIN_BLOCKS").ok().and_then(|v| v.parse().ok())
    {
        config.retain_blocks = blocks;
    }
    let store = reth_state_store_qmdb::QmdbStateStore::open(config.clone())
        .unwrap_or_else(|error| panic!("opening QMDB state store at {config:?}: {error}"));
    let tip = reth_state_store::StateStore::tip(&store).expect("QMDB state store tip");
    eprintln!("QMDB state store at {}: tip {tip:?}", config.dir.display());
    reth_state_store::set_global(std::sync::Arc::new(reth_state_store::ByHash::new(store)))
        .expect("state store installed once");
}

fn main() {
    base_cli_utils::init_common!();
    base_reth_cli::init_reth!();
    base_reth_cli::init_snapshots!();
    base_reth_cli::init_engine!();

    let cli = base_cli_utils::parse_cli!(NodeCli);

    install_state_store();

    cli.run(|builder, args| async move { StandardBaseRethNode::run(builder, args).await }).unwrap();
}
