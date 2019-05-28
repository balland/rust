use rustc::dep_graph::{DepGraph, /*DepKind,*/ WorkProduct, WorkProductId};
use rustc::session::Session;
use rustc::ty::TyCtxt;
use rustc::util::common::time;
use rustc_data_structures::fx::FxHashMap;
use rustc_data_structures::sync::join;
use rustc_serialize::Encodable as RustcEncodable;
use rustc_serialize::opaque::Encoder;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::PathBuf;

use super::data::*;
use super::fs::*;
use super::dirty_clean;
use super::file_format;
use super::work_product;

pub fn save_dep_graph<'a, 'tcx>(tcx: TyCtxt<'a, 'tcx, 'tcx>) {
    debug!("save_dep_graph()");
    tcx.dep_graph.with_ignore(|| {
        let sess = tcx.sess;
        if sess.opts.incremental.is_none() {
            return;
        }

        let query_cache_path = query_cache_path(sess);
        let temp_dep_graph_path = temp_dep_graph_path_from(&sess.incr_comp_session_dir());
        let dep_graph_path = dep_graph_path(sess);

        join(move || {
            if tcx.sess.opts.debugging_opts.incremental_queries {
                time(sess, "persist query result cache", || {
                    save_in(sess,
                            query_cache_path,
                            |e| encode_query_cache(tcx, e)).unwrap();
                });
            }
        }, || {
            time(sess, "swap dep-graph", || {
                swap_dep_graph(tcx, temp_dep_graph_path, dep_graph_path)
            });
        });

        dirty_clean::check_dirty_clean_annotations(tcx);
    })
}

pub fn save_work_product_index(sess: &Session,
                               dep_graph: &DepGraph,
                               new_work_products: FxHashMap<WorkProductId, WorkProduct>) {
    if sess.opts.incremental.is_none() {
        return;
    }

    debug!("save_work_product_index()");
    dep_graph.assert_ignored();
    let path = work_products_path(sess);
    save_in(sess, path, |e| encode_work_product_index(&new_work_products, e)).unwrap();

    // We also need to clean out old work-products, as not all of them are
    // deleted during invalidation. Some object files don't change their
    // content, they are just not needed anymore.
    let previous_work_products = dep_graph.previous_work_products();
    for (id, wp) in previous_work_products.iter() {
        if !new_work_products.contains_key(id) {
            work_product::delete_workproduct_files(sess, wp);
            debug_assert!(wp.saved_files.iter().all(|&(_, ref file_name)| {
                !in_incr_comp_dir_sess(sess, file_name).exists()
            }));
        }
    }

    // Check that we did not delete one of the current work-products:
    debug_assert!({
        new_work_products.iter()
                         .flat_map(|(_, wp)| wp.saved_files
                                               .iter()
                                               .map(|&(_, ref name)| name))
                         .map(|name| in_incr_comp_dir_sess(sess, name))
                         .all(|path| path.exists())
    });
}

pub(super) fn save_in<F>(sess: &Session, path_buf: PathBuf, encode: F) -> io::Result<File>
where
    F: FnOnce(&mut Encoder)
{
    debug!("save: storing data in {}", path_buf.display());

    // delete the old dep-graph, if any
    // Note: It's important that we actually delete the old file and not just
    // truncate and overwrite it, since it might be a shared hard-link, the
    // underlying data of which we don't want to modify
    if path_buf.exists() {
        match fs::remove_file(&path_buf) {
            Ok(()) => {
                debug!("save: remove old file");
            }
            Err(err) => {
                sess.err(&format!("unable to delete old dep-graph at `{}`: {}",
                                  path_buf.display(),
                                  err));
                return Err(err);
            }
        }
    }

    // generate the data in a memory buffer
    let mut encoder = Encoder::new(Vec::new());
    file_format::write_file_header(&mut encoder);
    encode(&mut encoder);

    // write the data out
    let data = encoder.into_inner();
    let mut file = File::create(&path_buf)?;

    match file.write_all(&data) {
        Ok(_) => {
            debug!("save: data written to disk successfully");
        }
        Err(err) => {
            sess.err(&format!("failed to write dep-graph to `{}`: {}",
                              path_buf.display(),
                              err));
            return Err(err);
        }
    }

    Ok(file)
}

fn swap_dep_graph(tcx: TyCtxt<'_, '_, '_>, temp: PathBuf, old: PathBuf) {
    let sess = tcx.sess;
    // Encode the graph data.
    time(tcx.sess, "finish graph serialization", || {
        tcx.dep_graph.serialize();
    });

    // delete the old dep-graph, if any
    // Note: It's important that we actually delete the old file and not just
    // truncate and overwrite it, since it might be a shared hard-link, the
    // underlying data of which we don't want to modify
    if old.exists() {
        match fs::remove_file(&old) {
            Ok(()) => {
                debug!("save: remove old file");
            }
            Err(err) => {
                sess.err(&format!("unable to delete old dep-graph at `{}`: {}",
                                  old.display(),
                                  err));
                return;
            }
        }
    }

    fs::rename(temp, old).expect("unable to rename temp dep graph");


/*
    if tcx.sess.opts.debugging_opts.incremental_info {
        #[derive(Clone)]
        struct Stat {
            kind: DepKind,
            node_counter: u64,
            edge_counter: u64,
        }

        let total_node_count = serialized_graph.nodes.len();
        let total_edge_count: usize = serialized_graph.nodes.iter().map(|d| d.deps.len()).sum();

        let mut counts: FxHashMap<_, Stat> = FxHashMap::default();

        for node in serialized_graph.nodes.iter() {
            let stat = counts.entry(node.node.kind).or_insert(Stat {
                kind: node.node.kind,
                node_counter: 0,
                edge_counter: 0,
            });

            stat.node_counter += 1;
            stat.edge_counter += node.deps.len() as u64;
        }

        let mut counts: Vec<_> = counts.values().cloned().collect();
        counts.sort_by_key(|s| -(s.node_counter as i64));

        let percentage_of_all_nodes: Vec<f64> = counts.iter().map(|s| {
            (100.0 * (s.node_counter as f64)) / (total_node_count as f64)
        }).collect();

        let average_edges_per_kind: Vec<f64> = counts.iter().map(|s| {
            (s.edge_counter as f64) / (s.node_counter as f64)
        }).collect();

        println!("[incremental]");
        println!("[incremental] DepGraph Statistics");

        const SEPARATOR: &str = "[incremental] --------------------------------\
                                 ----------------------------------------------\
                                 ------------";

        println!("{}", SEPARATOR);
        println!("[incremental]");
        println!("[incremental] Total Node Count: {}", total_node_count);
        println!("[incremental] Total Edge Count: {}", total_edge_count);
        if let Some((total_edge_reads,
                     total_duplicate_edge_reads)) = tcx.dep_graph.edge_deduplication_data() {
            println!("[incremental] Total Edge Reads: {}", total_edge_reads);
            println!("[incremental] Total Duplicate Edge Reads: {}", total_duplicate_edge_reads);
        }
        println!("[incremental]");
        println!("[incremental]  {:<36}| {:<17}| {:<12}| {:<17}|",
                 "Node Kind",
                 "Node Frequency",
                 "Node Count",
                 "Avg. Edge Count");
        println!("[incremental] -------------------------------------\
                  |------------------\
                  |-------------\
                  |------------------|");

        for (i, stat) in counts.iter().enumerate() {
            println!("[incremental]  {:<36}|{:>16.1}% |{:>12} |{:>17.1} |",
                format!("{:?}", stat.kind),
                percentage_of_all_nodes[i],
                stat.node_counter,
                average_edges_per_kind[i]);
        }

        println!("{}", SEPARATOR);
        println!("[incremental]");
    }
*/
}

fn encode_work_product_index(work_products: &FxHashMap<WorkProductId, WorkProduct>,
                             encoder: &mut Encoder) {
    let serialized_products: Vec<_> = work_products
        .iter()
        .map(|(id, work_product)| {
            SerializedWorkProduct {
                id: id.clone(),
                work_product: work_product.clone(),
            }
        })
        .collect();

    serialized_products.encode(encoder).unwrap();
}

fn encode_query_cache(tcx: TyCtxt<'_, '_, '_>,
                      encoder: &mut Encoder) {
    time(tcx.sess, "serialize query result cache", || {
        tcx.serialize_query_result_cache(encoder).unwrap();
    })
}
