(function () {
  "use strict";

  angular
    .module('mnStatisticsDescriptionService', [])
    .factory('mnStatisticsDescriptionService', mnStatisticsDescriptionFactory);

  function mnStatisticsDescriptionFactory() {
    return {
      "kvGroups": {
        "Ops":
        ["ops","cmd_get","cmd_set","hit_ratio","delete_hits","cas_hits","ep_cache_miss_rate","couch_views_ops","ep_num_ops_del_meta","ep_num_ops_get_meta","ep_num_ops_set_meta","ep_ops_create","ep_ops_update","vb_active_ops_create","vb_pending_ops_create","vb_replica_ops_create","xdc_ops","curr_connections"],
        "Memory":
        ["mem_used","ep_kv_size","ep_meta_data_memory","ep_tmp_oom_errors","ep_mem_low_wat","ep_mem_high_wat","vb_active_itm_memory","vb_active_meta_data_memory","vb_pending_itm_memory","vb_pending_meta_data_memory","vb_replica_itm_memory","vb_replica_meta_data_memory"],
        "Disk":
        ["couch_total_disk_size","ep_cache_miss_rate","vb_avg_total_queue_age","avg_disk_update_time","avg_disk_commit_time","couch_docs_actual_disk_size","couch_views_actual_disk_size",
         "disk_write_queue","ep_bg_fetched","ep_data_read_failed","ep_data_write_failed","ep_num_value_ejects","ep_ops_create","ep_ops_update"],
        "vBuckets":
        ["ep_vb_total","vb_active_num","curr_items","vb_active_ops_create","vb_active_resident_items_ratio","vb_active_eject","vb_active_sync_write_accepted_count","vb_active_sync_write_committed_count","vb_active_sync_write_aborted_count","avg_active_timestamp_drift","ep_active_ahead_exceptions","vb_pending_num","vb_pending_curr_items","vb_pending_ops_create","vb_pending_resident_items_ratio","vb_pending_eject","vb_replica_num","vb_replica_curr_items","vb_replica_ops_create","vb_replica_resident_items_ratio","vb_replica_eject","avg_replica_timestamp_drift","ep_replica_ahead_exceptions"],
        "Disk Queues":
        ["ep_diskqueue_fill","ep_diskqueue_drain","ep_diskqueue_items","vb_active_queue_fill","vb_active_queue_drain","vb_active_queue_size","vb_replica_queue_fill","vb_replica_queue_drain","vb_replica_queue_size","vb_pending_queue_fill","vb_pending_queue_drain","vb_pending_queue_size"],
        "DCP Queues":
        ["ep_dcp_views+indexes_count","ep_dcp_views+indexes_producer_count","ep_dcp_views+indexes_items_remaining","ep_dcp_views+indexes_total_bytes","ep_dcp_views+indexes_backoff","ep_dcp_cbas_count","ep_dcp_cbas_producer_count","ep_dcp_cbas_items_remaining","ep_dcp_cbas_total_bytes","ep_dcp_cbas_backoff","ep_dcp_replica_count","ep_dcp_replica_producer_count","ep_dcp_replica_items_remaining","ep_dcp_replica_total_bytes","ep_dcp_replica_backoff","ep_dcp_xdcr_count","ep_dcp_xdcr_producer_count","ep_dcp_xdcr_items_remaining","ep_dcp_xdcr_total_bytes","ep_dcp_xdcr_backoff","ep_dcp_eventing_count","ep_dcp_eventing_producer_count","ep_dcp_eventing_items_remaining","ep_dcp_eventing_total_bytes","ep_dcp_eventing_backoff"]
      },

      "stats": {
        "@system":{
          "cpu_cores_available": null,
          "cpu_idle_ms": null,
          "cpu_local_ms": null,
          "cpu_utilization_rate": {
            unit: "percent",
            title: "CPU",
            desc: "Percentage of CPU in use across all available cores on this server."
          },
          "hibernated_requests": {
            unit: "number",
            title: "Idle Streaming Requests",
            desc: "Number of streaming requests on management port (usually 8091) now idle."
          },
          "hibernated_waked": {
            unit: "number/sec",
            title: "Streaming Wakeups",
            desc: "Number of streaming request wakeups per second on management port (usually 8091)."
          },
          "mem_actual_free": {
            unit: "bytes",
            title: "Available RAM",
            desc: "Bytes of RAM available to Couchbase on this server."
          },
          "mem_actual_used": null,
          "mem_free": null,
          "mem_limit": null,
          "mem_total": null,
          "mem_used_sys": null,
          "rest_requests": {
            unit: "number/sec",
            title: "HTTP Request Rate",
            desc: "Number of http requests per second on management port (usually 8091)."
          },
          "swap_total": null,
          "swap_used": {
            unit: "bytes",
            title: "Swap Used",
            desc: "Bytes of swap space in use on this server."
          },
        },

        "@kv-": {
          "couch_total_disk_size": {
            unit: "bytes",
            title: "Data/Views On Disk",
            desc: "The total size on disk of all data and view files for this bucket. (measured from couch_total_disk_size)"
          },
          "couch_docs_fragmentation": {
            unit: "percent",
            title: "Docs Fragmentation",
            desc: "Percentage of fragmented data to be compacted compared to real data for the data files in this bucket. (measured from couch_docs_fragmentation)"
          },
          "couch_views_fragmentation": {
            unit: "percent",
            title: "Views Fragmentation",
            desc: "Percentage of fragmented data to be compacted compared to real data for the view index files in this bucket. (measured from couch_views_fragmentation)"
          },
          "hit_ratio": {
            unit: "percent",
            title: "Get Ratio",
            desc: "Percentage of get requests served with data from this bucket. (measured from get_hits * 100/cmd_get)"
          },
          "ep_cache_miss_rate": {
            unit: "percent",
            title: "Cache Miss Ratio",
            desc: "Percentage of reads per second to this bucket from disk as opposed to RAM. (measured from ep_bg_fetches / gets * 100)"
          },
          "ep_resident_items_rate": {
            unit: "percent",
            title: "Resident Ratio",
            desc: "Percentage of all items cached in RAM in this bucket. (measured from ep_resident_items_rate)"
          },
          "vb_avg_active_queue_age": {
            unit: "second",
            title: "Active Queue Age",
            desc: "Average age in seconds of active items in the active item queue for this bucket. (measured from vb_avg_active_queue_age)"
          },
          "vb_avg_replica_queue_age": {
            unit: "second",
            title: "Replica Queue Age",
            desc: "Average age in seconds of replica items in the replica item queue for this bucket. (measured from vb_avg_replica_queue_age)"
          },
          "vb_avg_pending_queue_age": {
            unit: "second",
            title: "Pending Queue Age",
            desc: "Average age in seconds of pending items in the pending item queue for this bucket. Should be transient during rebalancing. (measured from vb_avg_pending_queue_age)"
          },
          "vb_avg_total_queue_age": {
            unit: "second",
            title: "Disk Write Queue Age",
            desc: "Average age in seconds of all items in the disk write queue for this bucket. (measured from vb_avg_total_queue_age)"
          },
          "vb_active_resident_items_ratio": {
            unit: "percent",
            title: "Active Resident Ratio",
            desc: "Percentage of active items cached in RAM in this bucket. (measured from vb_active_resident_items_ratio)"
          },
          "vb_replica_resident_items_ratio": {
            unit: "percent",
            title: "Replica Resident Ratio",
            name: "vb_replica_resident_items_ratio",
            desc: "Percentage of replica items cached in RAM in this bucket. (measured from vb_replica_resident_items_ratio)"
          },
          "vb_pending_resident_items_ratio": {
            unit: "percent",
            title: "Pending Resident Ratio",
            desc: "Percentage of items cached in RAM for pending vBuckets in this bucket. (measured from vb_pending_resident_items_ratio)"
          },
          "avg_disk_update_time": {
            unit: "microsecond",
            title: "Disk Update Time",
            desc: "Average disk update time in microseconds as from disk_update histogram of timings. (measured from avg_disk_update_time)"
          },
          "avg_disk_commit_time": {
            unit: "percent",
            title: "Disk Commit Time",
            desc: "Average disk commit time in seconds as from disk_update histogram of timings. (measured from avg_disk_commit_time)"
          },
          "avg_bg_wait_time": {
            unit: "microsecond",
            title: "Background Fetch Time",
            desc: "Average background fetch time in microseconds. (measured from avg_bg_wait_time)"
          },
          "avg_active_timestamp_drift": {
            unit: "second",
            title: "Active Timestamp Drift",
            name: "avg_active_timestamp_drift",
            desc: "Average drift (in seconds) between mutation timestamps and the local time for active vBuckets. (measured from ep_active_hlc_drift and ep_active_hlc_drift_count)"
          },
          "avg_replica_timestamp_drift": {
            unit: "second",
            title: "Replica Timestamp Drift",
            desc: "Average drift (in seconds) between mutation timestamps and the local time for replica vBuckets. (measured from ep_replica_hlc_drift and ep_replica_hlc_drift_count)"
          },
          "ep_dcp_views+indexes_count": {
            unit: "number",
            title: "DCP Indexes Connections",
            desc: "Number of internal views/gsi/search index DCP connections in this bucket (measured from ep_dcp_views_count + ep_dcp_2i_count + ep_dcp_fts_count)"
          },
          "ep_dcp_views+indexes_items_remaining": {
            unit: "number",
            title: "DCP Indexes Items Remaining",
            desc: "Number of items remaining to be sent to consumer in this bucket (measured from ep_dcp_views_items_remaining + ep_dcp_2i_items_remaining + ep_dcp_fts_items_remaining)"
          },
          "ep_dcp_views+indexes_producer_count": {
            unit: "number",
            title: "DCP Indexes Senders",
            desc: "Number of views/gsi/search index senders for this bucket (measured from ep_dcp_views_producer_count + ep_dcp_2i_producer_count + ep_dcp_fts_producer_count)"
          },
          "ep_dcp_views+indexes_total_backlog_size": null,
          "ep_dcp_views+indexes_items_sent": {
            unit: "number/sec",
            title: "DCP Indexes Items Sent",
            desc: "Number of items per second being sent for a producer for this bucket (measured from ep_dcp_views_items_sent + ep_dcp_2i_items_sent + ep_dcp_fts_items_sent)"
          },
          "ep_dcp_views+indexes_total_bytes": {
            unit: "bytes/sec",
            title: "DCP Indexes Drain Rate",
            desc: "Number of bytes per second being sent for views/gsi/search index DCP connections for this bucket (measured from ep_dcp_views_total_bytes + ep_dcp_2i_total_bytes + ep_dcp_fts_total_bytes)"
          },
          "ep_dcp_views+indexes_backoff": {
            unit: "number/sec",
            title: "DCP Indexes Backoffs",
            desc: "Number of backoffs for views/gsi/search index DCP connections (measured from ep_dcp_views_backoff + ep_dcp_2i_backoff + ep_dcp_fts_backoff)"
          },
          "bg_wait_count": null,
          "bg_wait_total": null,
          "bytes_read": {
            unit: "bytes/sec",
            name: "bytes_read",
            title: "Memcached RX Rate",
            desc: "Bytes per second received in this bucket. (measured from bytes_read)"
          },
          "bytes_written": {
            unit: "bytes/sec",
            title: "Memcached TX Rate",
            desc: "Number of bytes per second sent from this bucket. (measured from bytes_written)"
          },
          "cas_badval": {
            unit: "number/sec",
            title: "CAS Badval Rate",
            desc: "Number of CAS operations per second using an incorrect CAS ID for data that this bucket contains. (measured from cas_badval)"
          },
          "cas_hits": {
            unit: "number/sec",
            title: "CAS Ops Rate",
            desc: "Number of operations with a CAS id per second for this bucket. (measured from cas_hits)"
            // memcached_stats_description
            // title: "CAS hits per sec.",
            // desc: "Number of CAS operations per second for data that this bucket contains (measured from cas_hits)"
          },
          "cas_misses": {
            unit: "number/sec",
            title: "CAS Miss Rate",
            desc: "Number of CAS operations per second for data that this bucket does not contain. (measured from cas_misses)"
          },
          "cmd_get": {
            unit: "number/sec",
            title: "Gets",
            desc: "Number of reads (get operations) per second from this bucket. (measured from cmd_get)"
            // memcached_stats_description
            // title: "gets per sec.",
            // desc: "Number of get operations serviced by this bucket (measured from cmd_get)"
          },
          "cmd_total_gets": {
            unit: "number/sec",
            title: "Total Gets",
            desc: "Number of total get operations per second from this bucket (measured from cmd_total_gets). This includes additional get operations such as get locked that are not included in cmd_get"
          },
          "cmd_set": {
            unit: "number/sec",
            title: "Sets",
            desc: "Number of writes (set operations) per second to this bucket. (measured from cmd_set)"
            // memcached_stats_description
            // title: "sets per sec.",
            // desc: "Number of set operations serviced by this bucket (measured from cmd_set)"
          },
          "couch_docs_actual_disk_size": {
            unit: "bytes",
            title: "Data Total Disk Size",
            desc: "The size of all data service files on disk for this bucket, including the data itself, metadata, and temporary files. (measured from couch_docs_actual_disk_size)"
          },
          "couch_docs_data_size": {
            unit: "bytes",
            title: "Active Data Size",
            desc: "Bytes of active data in this bucket. (measured from couch_docs_data_size)"
          },
          "couch_docs_disk_size": null,
          "couch_spatial_data_size": null,
          "couch_spatial_disk_size": null,
          "couch_spatial_ops": null,
          "couch_views_actual_disk_size": {
            unit: "bytes",
            title: "Views Disk Size",
            desc: "Bytes of active items in all the views for this bucket on disk (measured from couch_views_actual_disk_size)"
          },
          "couch_views_data_size": {
            unit: "bytes",
            title: "Views Data",
            desc: "Bytes of active data for all the views in this bucket. (measured from couch_views_data_size)"
          },
          "couch_views_disk_size": null,
          "couch_views_ops": {
            unit: "number/sec",
            title: "Views Read Rate",
            desc: "All the views reads for all design documents including scatter gather. (measured from couch_views_ops)"
          },
          "curr_connections": {
            unit: "number",
            title: "Current Connections",
            desc: "Number of currrent connections to this server including connections from external client SDKs, proxies, DCP requests and internal statistic gathering. (measured from curr_connections)"
          },
          "curr_items": {
            unit: "number",
            title: "Active Items",
            desc: "Number of active items in this bucket. (measured from curr_items)",
            //membase_vbucket_resources_stats_description
            //desc: "Number of items in \"active\" vBuckets in this bucket (measured from curr_items)"
            //memcached_stats_description
            //desc: "Number of items stored in this bucket (measured from curr_items)"
          },
          "curr_items_tot": {
            unit: "number",
            title: "Total Items",
            desc: "Total number of items in this bucket. (measured from curr_items_tot)"
          },
          "decr_hits": {
            unit: "number/sec",
            title: "Decr Hit Rate",
            desc: "Number of decrement operations per second for data that this bucket contains. (measured from decr_hits)"
          },
          "decr_misses": {
            unit: "number/sec",
            title: "Decr Miss Rate",
            desc: "Number of decr operations per second for data that this bucket does not contain. (measured from decr_misses)"
          },
          "delete_hits": {
            unit: "number/sec",
            title: "Delete Rate",
            desc: "Number of delete operations per second for this bucket. (measured from delete_hits)"
            //memcached_stats_description
            //title: "delete hits per sec.",
            //desc: "Number of delete operations per second for data that this bucket contains (measured from delete_hits)"
          },
          "delete_misses": {
            unit: "number/sec",
            title: "Delete Miss Rate",
            desc: "Number of delete operations per second for data that this bucket does not contain. (measured from delete_misses)"
          },
          "disk_commit_count": null,
          "disk_commit_total": null,
          "disk_update_count": null,
          "disk_update_total": null,
          "disk_write_queue": {
            unit: "number",
            title: "Disk Write Queue",
            desc: "Number of items waiting to be written to disk in this bucket. (measured from ep_queue_size+ep_flusher_todo)"
          },
          "ep_active_ahead_exceptions": {
            unit: "number/sec",
            title: "Active Ahead Exception Rate",
            desc: "Total number of ahead exceptions (when timestamp drift between mutations and local time has exceeded 5000000 μs) per second for all active vBuckets."
          },
          "ep_active_hlc_drift": null,
          "ep_active_hlc_drift_count": null,
          "ep_bg_fetched": {
            unit: "number/sec",
            title: "Disk Read Rate",
            desc: "Number of reads per second from disk for this bucket. (measured from ep_bg_fetched)"
          },
          "ep_clock_cas_drift_threshold_exceeded": null,
          "ep_data_read_failed": {
            unit: "number",
            title: "Disk Read Failures",
            desc: "Number of disk read failures. (measured from ep_data_read_failed)"
          },
          "ep_data_write_failed": {
            unit: "number",
            title: "Disk Write Failures",
            desc: "Number of disk write failures. (measured from ep_data_write_failed)"
          },
          "ep_dcp_2i_backoff": null,
          "ep_dcp_2i_count": null,
          "ep_dcp_2i_items_remaining": null,
          "ep_dcp_2i_items_sent": null,
          "ep_dcp_2i_producer_count": null,
          "ep_dcp_2i_total_backlog_size": null,
          "ep_dcp_2i_total_bytes": null,
          "ep_dcp_cbas_backoff": {
            unit: "number/sec",
            title: "DCP Analytics Backoffs",
            desc: "Number of backoffs per second for analytics DCP connections (measured from ep_dcp_cbas_backoff)"
          },
          "ep_dcp_cbas_count": {
            unit: "number",
            title: "DCP Analytics Connections",
            desc: "Number of internal analytics DCP connections in this bucket (measured from ep_dcp_cbas_count)"
          },
          "ep_dcp_cbas_items_remaining": {
            unit: "number",
            title: "DCP Analytics Items Remaining",
            desc: "Number of items remaining to be sent to consumer in this bucket (measured from ep_dcp_cbas_items_remaining)"
          },
          "ep_dcp_cbas_items_sent": {
            unit: "number/sec",
            title: "DCP Analytics Items Sent",
            desc: "Number of items per second being sent for a producer for this bucket (measured from ep_dcp_cbas_items_sent)"
          },
          "ep_dcp_cbas_producer_count": {
            unit: "number",
            title: "DCP Analytics Senders",
            desc: "Number of analytics senders for this bucket (measured from ep_dcp_cbas_producer_count)"
          },
          "ep_dcp_cbas_total_backlog_size": null,
          "ep_dcp_cbas_total_bytes": {
            unit: "bytes/sec",
            title: "DCP Analytics Drain Rate",
            desc:"Number of bytes per second being sent for analytics DCP connections for this bucket (measured from ep_dcp_cbas_total_bytes)"
          },
          "ep_dcp_fts_backoff": null,
          "ep_dcp_fts_count": null,
          "ep_dcp_fts_items_remaining": null,
          "ep_dcp_fts_items_sent": null,
          "ep_dcp_fts_producer_count": null,
          "ep_dcp_fts_total_backlog_size": null,
          "ep_dcp_fts_total_bytes": null,
          "ep_dcp_eventing_backoff": {
            unit: "number/sec",
            title: "DCP Eventing Backoffs",
            desc: "Number of backoffs per second for eventing DCP connections (measured from ep_dcp_eventing_backoff)"
          },
          "ep_dcp_eventing_count": {
            unit: "number",
            title: "DCP Eventing Connections",
            desc: "Number of internal eventing DCP connections in this bucket (measured from ep_dcp_eventing_count)"
          },
          "ep_dcp_eventing_items_remaining": {
            unit: "number",
            title: "DCP Eventing Items Remaining",
            desc: "Number of items remaining to be sent to consumer in this bucket (measured from ep_dcp_eventing_items_remaining)"
          },
          "ep_dcp_eventing_items_sent": {
            unit: "number/sec",
            title: "DCP Eventing Items Sent",
            desc: "Number of items per second being sent for a producer for this bucket (measured from ep_dcp_eventing_items_sent)"
          },
          "ep_dcp_eventing_producer_count": {
            unit: "number",
            title: "DCP Eventing Senders",
            desc: "Number of eventing senders for this bucket (measured from ep_dcp_eventing_producer_count)"
          },
          "ep_dcp_eventing_total_backlog_size": null,
          "ep_dcp_eventing_total_bytes": {
            unit: "bytes/sec",
            title: "DCP Eventing Drain Rate",
            desc:"Number of bytes per second being sent for eventing DCP connections for this bucket (measured from ep_dcp_eventing_total_bytes)"
          },
          "ep_dcp_other_backoff": {
            unit: "number/sec",
            title: "DCP Other Backoffs",
            desc: "Number of backoffs for other DCP connections"
          },
          "ep_dcp_other_count": {
            unit: "number",
            title: "DCP Other Connections",
            desc: "Number of other DCP connections in this bucket (measured from ep_dcp_other_count)"
          },
          "ep_dcp_other_items_remaining": {
            unit: "number",
            title: "DCP Other Items Remaining",
            desc: "Number of items remaining to be sent to consumer in this bucket (measured from ep_dcp_other_items_remaining)"
          },
          "ep_dcp_other_items_sent": {
            unit: "number/sec",
            title: "DCP Other Items Sent",
            desc: "Number of items per second being sent for a producer for this bucket (measured from ep_dcp_other_items_sent)"
          },
          "ep_dcp_other_producer_count": {
            unit: "number",
            title: "DCP Other Senders",
            desc: "Number of other senders for this bucket (measured from ep_dcp_other_producer_count)"
          },
          "ep_dcp_other_total_backlog_size": null,
          "ep_dcp_other_total_bytes": {
            unit: "bytes/sec",
            title: "DCP Other Drain Rate",
            desc: "Number of bytes per second being sent for other DCP connections for this bucket (measured from ep_dcp_other_total_bytes)"
          },
          "ep_dcp_replica_backoff": {
            unit: "number",
            title: "DCP Replication Backoffs",
            desc: "Number of backoffs for replication DCP connections"
          },
          "ep_dcp_replica_count": {
            unit: "number",
            title: "DCP Replication Connections",
            desc: "Number of internal replication DCP connections in this bucket (measured from ep_dcp_replica_count)"
          },
          "ep_dcp_replica_items_remaining": {
            unit: "number",
            title: "DCP Replication Items Remaining",
            desc: "Number of items remaining to be sent to consumer in this bucket (measured from ep_dcp_replica_items_remaining)",
            // membase_dcp_queues_stats_description
            // title: "items remaining",
            // desc: "Number of items remaining to be sent to consumer in this bucket (measured from ep_dcp_replica_items_remaining)"
          },
          "ep_dcp_replica_items_sent": {
            unit: "number",
            title: "DCP Replication Items Sent",
            desc: "Number of items per second being sent for a producer for this bucket (measured from ep_dcp_replica_items_sent)"
          },
          "ep_dcp_replica_producer_count": {
            unit: "number",
            title: "DCP Replication Senders",
            desc: "Number of replication senders for this bucket (measured from ep_dcp_replica_producer_count)"
          },
          "ep_dcp_replica_total_backlog_size": null,
          "ep_dcp_replica_total_bytes": {
            unit: "bytes/sec",
            title: "DCP Replication Drain Rate",
            desc: "Number of bytes per second being sent for replication DCP connections for this bucket (measured from ep_dcp_replica_total_bytes)"
          },
          "ep_dcp_views_backoff": null,
          "ep_dcp_views_count": null,
          "ep_dcp_views_items_remaining": null,
          "ep_dcp_views_items_sent": null,
          "ep_dcp_views_producer_count": null,
          "ep_dcp_views_total_backlog_size": null,
          "ep_dcp_views_total_bytes": null,
          "ep_dcp_xdcr_backoff": {
            unit: "number",
            title: "DCP XDCR Backoffs",
            desc: "Number of backoffs for XDCR DCP connections"
          },
          "ep_dcp_xdcr_count": {
            unit: "number",
            title: "DCP XDCR Connections",
            desc: "Number of internal XDCR DCP connections in this bucket (measured from ep_dcp_xdcr_count)"
          },
          "ep_dcp_xdcr_items_remaining": {
            unit: "number",
            title: "DCP XDCR Items Remaining",
            desc: "Number of items remaining to be sent to consumer in this bucket (measured from ep_dcp_xdcr_items_remaining)"
          },
          "ep_dcp_xdcr_items_sent": {
            unit: "number/sec",
            title: "DCP XDCR Items Sent",
            desc: "Number of items per second being sent for a producer for this bucket (measured from ep_dcp_xdcr_items_sent)"
          },
          "ep_dcp_xdcr_producer_count": {
            unit: "number",
            title: "DCP XDCR Senders",
            desc: "Number of XDCR senders for this bucket (measured from ep_dcp_xdcr_producer_count)"
          },
          "ep_dcp_xdcr_total_backlog_size": null,
          "ep_dcp_xdcr_total_bytes": {
            unit: "bytes/sec",
            title: "DCP XDCR Drain Rate",
            desc: "Number of bytes per second being sent for XDCR DCP connections for this bucket (measured from ep_dcp_xdcr_total_bytes)"
          },
          "ep_diskqueue_drain": {
            unit: "number/sec",
            title: "Disk Queue Total Drain Rate",
            desc: "Total number of items per second being written to disk in this bucket (measured from ep_diskqueue_drain)"
          },
          "ep_diskqueue_fill": {
            unit: "number/sec",
            title: "Disk Queue Total Fill Rate",
            desc: "Total number of items per second being put on the disk queue in this bucket (measured from ep_diskqueue_fill)"
          },
          "ep_diskqueue_items": {
            unit: "number",
            title: "Disk Queue Total Items",
            desc: "Total number of items waiting (in queue) to be written to disk in this bucket (measured from ep_diskqueue_items)"
          },
          "ep_flusher_todo": null,
          "ep_item_commit_failed": null,
          "ep_kv_size": {
            unit: "bytes",
            title: "User Data in RAM",
            desc: "Total amount of user data cached in RAM in this bucket. (measured from ep_kv_size)"
          },
          "ep_max_size": null,
          "ep_mem_high_wat": {
            unit: "bytes",
            title: "High Water Mark",
            desc: "High water mark (in bytes) for auto-evictions. (measured from ep_mem_high_wat)"
          },
          "ep_mem_low_wat": {
            unit: "bytes",
            title: "Low Water Mark",
            desc: "Low water mark (in bytes) for auto-evictions. (measured from ep_mem_low_wat)"
          },
          "ep_meta_data_memory": {
            unit: "bytes",
            title: "Total Metadata in RAM",
            desc: "Bytes of item metadata consuming RAM in this bucket (measured from ep_meta_data_memory)"
          },
          "ep_num_non_resident": null,
          "ep_num_ops_del_meta": {
            unit: "number/sec",
            title: "XDCR Incoming Delete Rate",
            desc: "Number of delete operations per second for this bucket as the target for XDCR. (measured from ep_num_ops_del_meta)"
          },
          "ep_num_ops_del_ret_meta": null,
          "ep_num_ops_get_meta": {
            unit: "number/sec",
            title: "XDCR Incoming Metadata Read Rate",
            desc: "Number of metadata read operations per second for this bucket as the target for XDCR. (measured from ep_num_ops_get_meta)"
          },
          "ep_num_ops_set_meta": {
            unit: "number/sec",
            title: "XDCR Incoming Set Rate",
            desc: "Number of set operations per second for this bucket as the target for XDCR. (measured from ep_num_ops_set_meta)"
          },
          "ep_num_ops_set_ret_meta": null,
          "ep_num_value_ejects": {
            unit: "number/sec",
            title: "Ejection Rate",
            desc: "Number of items per second being ejected to disk in this bucket. (measured from ep_num_value_ejects)"
          },
          "ep_oom_errors": null,
          "ep_ops_create": {
            unit: "number/sec",
            title: "Total Disk Create Rate",
            desc: "Number of new items created on disk per second for this bucket. (measured from vb_active_ops_create + vb_replica_ops_create + vb_pending_ops_create)"
            // membase_vbucket_resources_stats_description
            // title: "new items per sec.",
            // desc: "Total number of new items being inserted into this bucket (measured from ep_ops_create)"
          },
          "ep_ops_update": {
            unit: "number/sec",
            title: "Disk Update Rate",
            desc: "Number of items updated on disk per second for this bucket. (measured from vb_active_ops_update + vb_replica_ops_update + vb_pending_ops_update)"
          },
          "ep_overhead": null,
          "ep_queue_size": null,
          "ep_replica_ahead_exceptions": {
            unit: "number/sec",
            title: "Replica Ahead Exception Rate",
            desc: "Total number of ahead exceptions (when timestamp drift between mutations and local time has exceeded 5000000 μs) per second for all replica vBuckets."
          },
          "ep_replica_hlc_drift": null,
          "ep_replica_hlc_drift_count": null,
          "ep_tmp_oom_errors": {
            unit: "number/sec",
            title: "Temp OOM Rate",
            desc: "Number of back-offs sent per second to client SDKs due to \"out of memory\" situations from this bucket. (measured from ep_tmp_oom_errors)"
          },
          "ep_vb_total": {
            unit: "number",
            title: "vBuckets Total",
            desc: "Total number of vBuckets for this bucket. (measured from ep_vb_total)"
          },
          "evictions": {
            unit: "number/sec",
            title: "Eviction Rate",
            desc: "Number of items per second evicted from this bucket. (measured from evictions)"
          },
          "get_hits": {
            unit: "number/sec",
            title: "Get Hit Rate",
            desc: "Number of get operations per second for data that this bucket contains. (measured from get_hits)"
          },
          "get_misses": {
            unit: "number/sec",
            title: "Get Miss Rate",
            desc: "Number of get operations per second for data that this bucket does not contain. (measured from get_misses)",
          },
          "incr_hits": {
            unit: "number/sec",
            title: "Incr Hit Rate",
            desc: "Number of increment operations per second for data that this bucket contains. (measured from incr_hits)"
          },
          "incr_misses": {
            unit: "number/sec",
            title: "Incr Miss Rate",
            desc: "Number of increment operations per second for data that this bucket does not contain. (measured from incr_misses)"
          },
          "mem_used": {
            unit: "bytes",
            title: "Data Total RAM Used",
            desc: "Total memory used in bytes. (as measured from mem_used)"
            // memcached_stats_description
            // isBytes: true
            // title: "RAM used",
            // desc: "Total amount of RAM used by this bucket (measured from mem_used)"
          },
          "misses": null,
          "ops": {
            unit: "number/sec",
            title: "Total Ops",
            desc: "Total operations per second (including XDCR) to this bucket. (measured from cmd_get + cmd_set + incr_misses + incr_hits + decr_misses + decr_hits + delete_misses + delete_hits + ep_num_ops_del_meta + ep_num_ops_get_meta + ep_num_ops_set_meta)"
            // memcached_stats_description
            // title: "ops per sec.",
            // default: true,
            // desc: "Total operations per second serviced by this bucket (measured from cmd_get + cmd_set + incr_misses + incr_hits + decr_misses + decr_hits + delete_misses + delete_hits + get_meta + set_meta + delete_meta)"
          },
          "vb_active_eject": {
            unit: "number/sec",
            title: "Active Ejection Rate",
            desc: "Number of items per second being ejected to disk from active vBuckets in this bucket. (measured from vb_active_eject)"
          },
          "vb_active_itm_memory": {
            unit: "bytes",
            title: "Active User Data in RAM",
            desc: "Amount of active user data cached in RAM in this bucket. (measured from vb_active_itm_memory)"
          },
          "vb_active_meta_data_memory": {
            unit: "bytes",
            title: "Active Metadata in RAM",
            desc: "Amount of active item metadata consuming RAM in this bucket. (measured from vb_active_meta_data_memory)"
          },
          "vb_active_num": {
            unit: "number",
            title: "vBuckets Active",
            desc: "Number of active vBuckets in this bucket. (measured from vb_active_num)"
          },
          "vb_active_num_non_resident": null,
          "vb_active_ops_create": {
            unit: "number/sec",
            title: "Active Create Rate",
            desc: "New items per second being inserted into active vBuckets in this bucket. (measured from vb_active_ops_create)"
          },
          "vb_active_ops_update": null,
          "vb_active_queue_age": null,
          "vb_active_queue_drain": {
            unit: "number/sec",
            title: "Disk Queue Active Drain Rate",
            desc: "Number of active items per second being written to disk in this bucket. (measured from vb_active_queue_drain)"
          },
          "vb_active_queue_fill": {
            unit: "number/sec",
            title: "Disk Queue Active Fill Rate",
            desc: "Number of active items per second being put on the active item disk queue in this bucket. (measured from vb_active_queue_fill)"
          },
          "vb_active_queue_size": {
            unit: "number",
            title: "Disk Queue Active Items",
            desc: "Number of active items waiting to be written to disk in this bucket. (measured from vb_active_queue_size)"
          },
          "vb_active_sync_write_accepted_count": {
            unit: "number/sec",
            title: "Accepted Sync Writes Rate",
            desc: "Number of accepted synchronous write per second into active vBuckets in this bucket. (measured from vb_active_sync_write_accepted_count)"
          },
          "vb_active_sync_write_committed_count": {
            unit: "number/sec",
            title: "Committed Sync Writes Rate",
              desc: "Number of committed synchronous writes per second into active vBuckets in this bucket. (measured from vb_active_sync_write_committed_count)"
          },
          "vb_active_sync_write_aborted_count": {
            unit: "number/sec",
            title: "Aborted Sync Writes Rate",
            desc: "Number of aborted synchronous writes per second into active vBuckets in this bucket. (measured from vb_active_sync_write_aborted_count)"
          },
          "vb_pending_curr_items": {
            unit: "number",
            title: "Pending Items",
            desc: "Number of items in pending vBuckets in this bucket. Should be transient during rebalancing. (measured from vb_pending_curr_items)"
          },
          "vb_pending_eject": {
            unit: "number/sec",
            title: "Pending Ejection Rate",
            desc: "Number of items per second being ejected to disk from pending vBuckets in this bucket. Should be transient during rebalancing. (measured from vb_pending_eject)"
          },
          "vb_pending_itm_memory": {
            unit: "bytes",
            title: "Pending User Data in RAM",
            desc: "Amount of pending user data cached in RAM in this bucket. Should be transient during rebalancing. (measured from vb_pending_itm_memory)"
          },
          "vb_pending_meta_data_memory": {
            unit: "bytes",
            title: "Pending Metadata in RAM",
            desc: "Amount of pending item metadata consuming RAM in this bucket. Should be transient during rebalancing. (measured from vb_pending_meta_data_memory)"
          },
          "vb_pending_num": {
            unit: "number",
            title: "vBuckets Pending",
            desc: "Number of pending vBuckets in this bucket. Should be transient during rebalancing. (measured from vb_pending_num)"
          },
          "vb_pending_num_non_resident": null,
          "vb_pending_ops_create": {
            unit: "number/sec",
            title: "Pending Create Rate",
            desc: "New items per second being instead into pending vBuckets in this bucket. Should be transient during rebalancing. (measured from vb_pending_ops_create)"
          },
          "vb_pending_ops_update": null,
          "vb_pending_queue_age": null,
          "vb_pending_queue_drain": {
            unit: "number/sec",
            title: "Disk Queue Pending Drain Rate",
            desc: "Number of pending items per second being written to disk in this bucket. Should be transient during rebalancing. (measured from vb_pending_queue_drain)"
          },
          "vb_pending_queue_fill": {
            unit: "number/sec",
            title: "Disk Queue Pending Fill Rate",
            desc: "Number of pending items per second being put on the pending item disk queue in this bucket. Should be transient during rebalancing (measured from vb_pending_queue_fill)"
          },
          "vb_pending_queue_size": {
            unit: "number",
            title: "Disk Queue Pending Items",
            desc: "Number of pending items waiting to be written to disk in this bucket and should be transient during rebalancing  (measured from vb_pending_queue_size)"
          },
          "vb_replica_curr_items": {
            unit: "number",
            title: "Replica Items",
            desc: "Number of items in replica vBuckets in this bucket. (measured from vb_replica_curr_items)"
          },
          "vb_replica_eject": {
            unit: "number/sec",
            title: "Replica Ejection Rate",
            desc: "Number of items per second being ejected to disk from replica vBuckets in this bucket. (measured from vb_replica_eject)"
          },
          "vb_replica_itm_memory": {
            unit: "bytes",
            title: "Replica User Data in RAM",
            desc: "Amount of replica user data cached in RAM in this bucket. (measured from vb_replica_itm_memory)"
          },
          "vb_replica_meta_data_memory": {
            unit: "bytes",
            title: "Replica Metadata in RAM",
            desc: "Amount of replica item metadata consuming in RAM in this bucket. (measured from vb_replica_meta_data_memory)"
          },
          "vb_replica_num": {
            unit: "number",
            title: "vBuckets Replica",
            desc: "Number of replica vBuckets in this bucket. (measured from vb_replica_num)"
          },
          "vb_replica_num_non_resident": null,
          "vb_replica_ops_create": {
            unit: "number/sec",
            title: "Replica Item Create Rate",
            desc: "New items per second being inserted into \"replica\" vBuckets in this bucket (measured from vb_replica_ops_create"
          },
          "vb_replica_ops_update": null,
          "vb_replica_queue_age": null,
          "vb_replica_queue_drain": {
            unit: "number/sec",
            title: "Disk Queue Replica Drain Rate",
            desc: "Number of replica items per second being written to disk in this bucket (measured from vb_replica_queue_drain)"
          },
          "vb_replica_queue_fill": {
            unit: "number/sec",
            title: "Disk Queue Replica Fill Rate",
            desc: "Number of replica items per second being put on the replica item disk queue in this bucket (measured from vb_replica_queue_fill)"
          },
          "vb_replica_queue_size": {
            unit: "number",
            title: "Disk Queue Replica Items",
            desc: "Number of replica items waiting to be written to disk in this bucket (measured from vb_replica_queue_size)"
          },
          "vb_total_queue_age": null,
          "xdc_ops": {
            unit: "number/sec",
            title: "XDCR Incoming Op Rate",
            desc: "Number of incoming XDCR operations per second for this bucket. (measured from xdc_ops)"
            // membase_incoming_xdcr_operations_stats_description
            // title: "total ops per sec.",
            // desc: "Total XDCR operations per second for this bucket (measured from ep_num_ops_del_meta + ep_num_ops_get_meta + ep_num_ops_set_meta)"
          },

          "@items": {
            "accesses": {
              unit: "number/sec",
              title: "Views Read Rate",
              desc: "Traffic to the views in this design doc."
            },
            "data_size": {
              unit: "bytes",
              title: "Views Data Size",
              desc: "Bytes stored in memory for views in this design doc."
            },
            "disk_size": {
              unit: "bytes",
              title: "Views Disk Size",
              desc: "Bytes stored on disk for views in this design doc."
            }
          }
        },

        "@index":{
          "index_memory_quota": null,
          "index_memory_used": null,
          "index_ram_percent": {
            unit: "percent",
            title: "Index RAM Quota Used",
            desc: "Percentage of Index RAM quota in use across all indexes on this server."
          },
          "index_remaining_ram": {
            unit: "bytes",
            title: "Index RAM Quota Available",
            desc: "Bytes of Index RAM quota still available on this server."
          }
        },

        "@index-":{
          "@items": {
            "num_docs_pending+queued": {
              unit: "number",
              title: "Index Mutations Remaining",
              desc: "Number of documents pending to be indexed. Per index."
            },
            "num_docs_indexed": {
              unit: "number/sec",
              title: "Index Drain Rate",
              desc: "Number of documents indexed by the indexer per second. Per index."
            },
            "index_resident_percent": {
              unit: "percent",
              title: "Index Resident Percent",
              desc: "Percentage of index data resident in memory. Per index."
            },
            "memory_used": {
              unit: "bytes",
              title: "Index RAM Used",
              desc: "Bytes in memory for this index. Per index."
            },
            "items_count": {
              unit: "number",
              title: "Indexed Items",
              desc: "Current total indexed documents. Per index."
            },
            "data_size": {
              unit: "bytes",
              title: "Index Data Size",
              desc: "Bytes of data in this index. Per index."
              //membase_index_stats_description
              //title: "index data size"
            },
            "disk_size": {
              unit: "bytes",
              title: "Index Disk Size",
              desc: "Bytes on disk for this index. Per index."
            },
            "avg_item_size": {
              unit: "bytes",
              title: "Index Item Size",
              desc: "Average size of each index item. Per index."
            },
            "avg_scan_latency": {
              unit: "nanoseconds",
              title: "Index Scan Latency",
              desc: "Average time (in nanoseconds) to serve a scan request. Per index."
            },
            "num_requests": {
              unit: "number/sec",
              title: "Index Request Rate",
              desc: "Number of requests served by the indexer per second. Per index."
            },
            "num_rows_returned": {
              unit: "number/sec",
              title: "Index Scan Items",
              desc: "Number of index items scanned by the indexer per second. Per index."
            },
            "scan_bytes_read": {
              unit: "number/sec",
              title: "Index Scan Bytes",
              desc: "Bytes per second read by a scan. Per index."
            },
            "cache_hits": null,
            "index_frag_percent": {
              unit: "percent",
              title: "Index Fragmentation",
              desc: "Percentage fragmentation of the index. Note: at small index sizes of less than a hundred kB, the static overhead of the index disk file will inflate the index fragmentation percentage. Per index."
            },
            "cache_miss_ratio": {
              unit: "percent",
              title: "Index Cache Miss Ratio",
              desc: "Percentage of accesses to this index data from disk as opposed to RAM. (measured from cache_misses * 100 / (cache_misses + cache_hits))"
            },
            "cache_misses": null,
            "disk_overhead_estimate": null,
            "frag_percent": null,
            "num_docs_pending": null,
            "num_docs_queued": null,
            "recs_in_mem": null,
            "recs_on_disk": null,
            "total_scan_duration": null,
          },
          "index/cache_hits": null,
          "index/cache_misses": null,
          "index/data_size": {
            unit: "bytes",
            title: "Index Total RAM Used",
            desc: "Bytes in memory used by Index across all indexes and buckets."
          },
          "index/disk_overhead_estimate": null,
          "index/disk_size": {
            unit: "bytes",
            title: "Index Total Disk Size",
            desc: "Bytes on disk used by Index across all indexes and buckets."
          },
          "index/frag_percent": null,
          "index/fragmentation": {
            unit: "percent",
            title: "Index Total Fragmentation",
            desc: "Percentage fragmentation for all indexes. Note: at small index sizes of less than a hundred kB, the static overhead of the index disk file will inflate the index fragmentation percentage."
          },
          "index/cache_hits": null,
          "index/cache_misses": null,
          "index/items_count": {
            unit: "number",
            title: "Index Doc Count",
            desc: "Current total number of indexed documents"
          },
          "index/memory_used": {
            unit: "bytes",
            title: "Index RAM Used",
            desc: "Total memory used by the index."
          },
          "index/num_docs_indexed": {
            unit: "number/sec",
            title: "Indexing Rate",
            desc: "Number of documents indexed by the indexer per second."
          },
          "index/num_docs_pending": null,
          "index/num_docs_queued": null,
          "index/num_requests": {
            unit: "number/sec",
            title: "Index Request Rate",
            desc: "Number of requests served by the indexer per second"
          },
          "index/num_rows_returned": {
            unit: "number/sec",
            title: "Index Total Scan Rate",
            desc: "Number of index items scanned by the indexer per second across all indexes."
          },
          "index/recs_in_mem": null,
          "index/recs_on_disk": null,
          "index/scan_bytes_read": {
            unit: "bytes/sec",
            title: "Index Scan Bytes",
            desc: "Number of bytes/sec scanned by the index."
          },
          "index/total_scan_duration": null
        },

        "@query":{
          "query_avg_req_time": {
            unit: "second",
            title: "Query Request Time",
            desc: "Average end-to-end time to process a query (in seconds)."
          },
          "query_avg_svc_time": {
            unit: "second",
            title: "Query Execution Time",
            desc: "Average time to execute a query (in seconds)."
          },
          "query_avg_response_size": {
            unit: "bytes",
            title: "Query Result Size",
            desc: "Average size (in bytes) of the data returned by a query"
          },
          "query_avg_result_count": {
            unit: "number",
            title: "Query Result Items",
            desc: "Average number of results (items/documents) returned by a query."
          },
          "query_active_requests": null,
          "query_errors": {
            unit: "number/sec",
            title: "N1QL Error Rate",
            desc: "Number of N1QL errors returned per second."
          },
          "query_invalid_requests": {
            unit: "number/sec",
            title: "N1QL Invalid Request Rate",
            desc: "Number of requests for unsupported endpoints per second, specifically HTTP requests for all endpoints not supported by the query engine. For example, a request for http://localhost:8093/foo will be included. Potentially useful in identifying DOS attacks."
          },
          "query_queued_requests": null,
          "query_request_time": null,
          "query_requests": {
            unit: "number/sec",
            title: "N1QL Request Rate",
            desc: "Number of N1QL requests processed per second."
            // membase_query_stats_description
            // title: "N1QL queries/sec"
            // desc: "Number of N1QL requests processed per second"
          },
          "query_requests_1000ms": {
            unit: "number/sec",
            title: "Queries > 1000ms",
            desc: "Number of queries that take longer than 1000 ms per second"
          },
          "query_requests_250ms": {
            unit: "number/sec",
            title: "Queries > 250ms",
            desc: "Number of queries that take longer than 250 ms per second."
          },
          "query_requests_5000ms": {
            unit: "number/sec",
            title: "Queries > 5000ms",
            desc: "Number of queries that take longer than 5000 ms per second."
          },
          "query_requests_500ms": {
            unit: "number/sec",
            title: "Queries > 500ms",
            desc: "Number of queries that take longer than 500 ms per second."
          },
          "query_result_count": null,
          "query_result_size": null,
          "query_selects": {
            unit: "number/sec",
            title: "N1QL Select Rate",
            desc: "Number of N1QL selects processed per second."
          },
          "query_service_time": null,
          "query_warnings": {
            unit: "number/sec",
            title: "N1QL Warning Rate",
            desc: "Number of N1QL warnings returned per second."
          }
        },

        "@fts-": {
          "@items": {
            "avg_queries_latency": {
              unit: "millisecond",
              title: "Search Query Latency",
              desc: "Average milliseconds to answer a Search query. Per index. (measured from avg_queries_latency)"
            },
            "doc_count": {
              unit: "number",
              title: "Search Docs",
              desc: "Number of documents examined. Per index. (measured from doc_count)"
            },
            "num_bytes_used_disk": {
              unit: "bytes",
              title:"Search Disk Size",
              desc: "Bytes on disk for this index. Per index. (measured from num_bytes_used_disk)"
            },
            "num_files_on_disk": {
              unit: "number",
              title: "Search Disk Files",
              desc: "Number of search files on disk across all partitions. (measured from num_files_on_disk)"
            },
            "num_root_memorysegments": {
              unit: "number",
              title: "Search Memory Segments",
              desc: "Number of memory segments in the index across all partitions. (measured from num_root_memorysegments)"
            },
            "num_root_filesegments": {
              unit: "number",
              title: "Search Disk Segments",
              desc: "Number of file segments in the index across all partitions. (measured from num_root_filesegments)"
            },
            "num_mutations_to_index": {
              unit: "number",
              title: "Search Mutations Remaining",
              desc: "Number of mutations not yet indexed. Per index. (measured from num_mutations_to_index)"
            },
            "num_pindexes_actual": {
              unit: "number",
              title: "Search Partitions",
              desc: "Number of index partitions. Per index. (including replica partitions, measured from num_pindexes_actual)"
            },
            "num_pindexes_target": {
              unit: "number",
              title: "Search Partitions Expected",
              desc: "Number of index partitions expected. Per index. (including replica partitions, measured from num_pindexes_target)"
            },
            "num_recs_to_persist": {
              unit: "number",
              title: "Search Records to Persist",
              desc: "Number of index records not yet persisted to disk. Per index. (measured from num_recs_to_persist)"
            },
            "total_bytes_indexed": {
              unit: "bytes/sec",
              title: "Search Index Rate",
              desc: "Bytes of plain text indexed per second. Per index. (measured from total_bytes_indexed)"
            },
            "total_bytes_query_results": {
              unit: "bytes/sec",
              title: "Search Result Rate",
              desc: "Bytes returned in results per second. Per index. (measured from total_bytes_query_results)"
            },
            "total_compaction_written_bytes": {
              unit: "bytes/sec",
              title: "Search Compaction Rate",
              desc: "Compaction bytes written per second. Per index. (measured from total_compaction_written_bytes)",
            },
            "total_queries": {
              unit: "number/sec",
              title: "Search Query Rate",
              desc: "Number of queries per second. Per index. (measured from total_queries)"
            },
            "total_queries_error": {
              unit: "number/sec",
              title: "Search Query Error Rate",
              desc: "Number of queries per second (including timeouts) that resulted in errors. Per index. (measured from total_queries_error)"
            },
            "total_queries_slow": {
              unit: "number/sec",
              title: "Search Slow Queries",
              desc: "Number of slow queries (> 5s to run) per second. Per index. (measured from total_queries_slow)"
            },
            "total_queries_timeout": {
              unit: "number/sec",
              title: "Search Query Timeout Rate",
              desc: "Number of queries that timeout per second. Per index. (measured from total_queries_timeout)"
            },
            "total_request_time": null,
            "total_term_searchers": {
              unit: "number/sec",
              title: "Term Searchers Start Rate",
              desc: "Number of term searchers started per second. Per index. (measured from total_term_searchers)"
            },
          },
          "fts/doc_count": null,
          "fts/num_bytes_used_disk": {
            unit: "bytes",
            title: "Search Total Disk Used",
            desc: "Bytes stored on disk for all Search indexes in this bucket."
          },
          "fts/num_files_on_disk": {
            unit: "number",
            title: "Search Disk Files",
            desc: "Number of search files on disk across all partitions."
          },
          "fts/num_mutations_to_index": null,
          "fts/num_pindexes_actual": null,
          "fts/num_pindexes_target": null,
          "fts/num_recs_to_persist": null,
          "fts/total_bytes_indexed": {
            unit: "bytes/sec",
            title: "Search Index Rate",
            desc: "Search bytes indexed per second for all Search indexes in this bucket."
          },
          "fts/total_bytes_query_results": null,
          "fts/total_compaction_written_bytes": null,
          "fts/total_queries": {
            unit: "number/sec",
            title: "Search Query Rate",
            desc: "Search queries per second for all Search indexes in this bucket."
          },
          "fts/total_queries_error": null,
          "fts/total_queries_slow": null,
          "fts/total_queries_timeout": null,
          "fts/total_request_time": null,
          "fts/total_term_searchers": null
        },

        "@fts": {
          "fts_num_bytes_used_ram": {
            unit: "bytes",
            title: "Search Total RAM Used",
            desc: "Bytes of RAM used by Search across all indexes and all buckets on this server."
          },
          "fts_total_queries_rejected_by_herder": {
            unit: "number",
            title: "Search Queries Rejected",
            desc: "Number of queries rejected by throttler due to high memory consumption."
          },
          "fts_curr_batches_blocked_by_herder": {
            unit: "number",
            title: "DCP batches blocked by FTS throttler",
            desc: "DCP batches blocked by throttler due to high memory consumption."
          }
        },

        "@cbas-":{
          "cbas/failed_at_parser_records_count": null,
          "cbas/failed_at_parser_records_count_total": {
            unit: "number",
            title: "Analytics Parse Fail Since Connect",
            desc: "Number of records Analytics failed to parse during bucket synchronization - since last bucket connect."
          },
          "cbas/incoming_records_count": {
            unit: "number/sec",
            title: "Analytics Ops Rate",
            desc: "Operations (gets + sets + deletes) per second processed by Analytics for this bucket."
          },
          "cbas/incoming_records_count_total": {
            unit: "number",
            title: "Analytics Ops Since Connect",
            desc: "Number of operations (gets + sets + deletes) processed by Analytics for this bucket since last bucket connect."
          }
        },

        "@cbas":{
          "cbas_disk_used": {
            unit: "bytes",
            title: "Analytics Total Disk Size",
            desc: "The total disk size used by Analytics."
          },
          "cbas_gc_count": {
            unit: "number",
            title: "Analytics Garbage Collection Rate",
            desc: "Number of JVM garbage collections per second for this Analytics node."
          },
          "cbas_gc_time": {
            unit: "millisecond/sec",
            title: "Analytics Garbage Collection Time",
            desc: "The amount of time in milliseconds spent performing JVM garbage collections for Analytics node."
          },
          "cbas_heap_used": {
            unit: "bytes",
            title: "Analytics Heap Used",
            desc: "Bytes of JVM heap used by Analytics on this server."
          },
          "cbas_system_load_average": {
            unit: "bytes",
            title: "Analytics System Load",
            desc: "System load in bytes for Analytics node."
          },
          "cbas_thread_count": {
            unit: "number",
            title: "Analytics Thread Count",
            desc: "Number of threads for Analytics node."
          },
          "cbas_io_reads": {
            unit: "bytes/sec",
            title: "Analytics Read Rate",
            desc: "Number of disk bytes read on Analytics node per second."
          },
          "cbas_io_writes": {
            unit: "bytes/sec",
            title: "Analytics Write Rate",
            desc: "Number of disk bytes written on Analytics node per second."
          }
        },

        "@eventing":{
          "eventing/processed_count": {
            unit: "number",
            title: "Eventing Mutations Processed",
            desc: "Mutations the function has finished processing. Per function."
          },
          "eventing/failed_count": {
            unit: "number",
            title: "Eventing Failures",
            desc: "Mutations for which the function execution failed. Per function."
          },
          "eventing/dcp_backlog": {
            unit: "number",
            title: "Eventing Backlog",
            desc: "Remaining mutations to be processed by the function. Per function."
          },
          "eventing/timeout_count": {
            unit: "number",
            title: "Eventing Timeouts",
            desc: "Execution timeouts while processing mutations. Per function."
          }
        },

        "@xdcr-":{
          "replication_changes_left": {
            unit: "number/sec",
            title: "XDCR Total Outbound Mutations",
            desc: "Number of mutations to be replicated to other clusters. (measured from replication_changes_left)"
          },
          "replication_docs_rep_queue": null,
          "@items": {
            "percent_completeness": {
              unit: "percent",
              title: "XDCR Checked Ratio",
              desc: "Percentage of checked items out of all checked and to-be-replicated items. Per-replication. (measured from percent_completeness)"
            },
            "bandwidth_usage": {
              unit: "bytes/sec",
              title: "XDCR Replication Rate",
              desc: "Rate of replication in terms of bytes replicated per second. Per-replication. (measured from bandwidth_usage)"
            },
            "changes_left": {
              unit: "number",
              title: "XDCR Replication Mutations",
              desc: "Number of mutations to be replicated to other clusters. Per-replication. (measured from changes_left)"
            },
            "data_replicated": null,
            "dcp_datach_length": null,
            "dcp_dispatch_time": null,
            "deletion_docs_written": null,
            "deletion_failed_cr_source": null,
            "deletion_filtered": null,
            "deletion_received_from_dcp": null,
            "docs_checked": null,
            "docs_failed_cr_source": {
              unit: "number",
              title: "XDCR Mutations Skipped",
              desc: "Number of mutations that failed conflict resolution on the source side and hence have not been replicated to other clusters. Per-replication. (measured from per-replication stat docs_failed_cr_source)"
            },
            "docs_filtered": {
              unit: "number/sec",
              title: "XDCR Mutations Filtered Rate",
              desc: "Number of mutations per second that have been filtered out and have not been replicated to other clusters. Per-replication. (measured from per-replication stat docs_filtered)"
            },
            "docs_opt_repd": null,
            "docs_processed": null,
            "docs_received_from_dcp": null,
            "docs_rep_queue": null,
            "docs_written": {
              unit: "number",
              title: "XDCR Mutations Replicated",
              desc: "Number of mutations that have been replicated to other clusters. Per-replication. (measured from docs_written)"
            },
            "expiry_docs_written": null,
            "expiry_failed_cr_source": null,
            "expiry_filtered": null,
            "expiry_received_from_dcp": null,
            "num_checkpoints": null,
            "num_failedckpts": null,
            "rate_doc_checks": {
              unit: "number/sec",
              title: "XDCR Doc Check Rate",
              desc: "Number of doc checks per second. Per-replication."
            },
            "rate_doc_opt_repd": {
              unit: "number/sec",
              title: "XDCR Optimistic Replication Rate",
              desc: "Number of replicated mutations per second. Per-replication."
            },
            "rate_received_from_dcp": {
              unit: "number/sec",
              title: "doc reception rate",
              desc: "Rate of mutations received from dcp in terms of number of mutations per second. Per-replication."
            },
            "rate_replicated": {
              unit: "number/sec",
              title: "XDCR Replication Rate",
              desc:"Number of replicated mutations per second. Per-replication. (measured from rate_replicated)"
            },
            "resp_wait_time": null,
            "set_docs_written": null,
            "set_failed_cr_source": null,
            "set_filtered": null,
            "set_received_from_dcp": null,
            "size_rep_queue": null,
            "throttle_latency": null,
            "time_committing": null,
            "wtavg_docs_latency": {
              unit: "millisecond",
              title: "XDCR Doc Batch Latency",
              desc: "Weighted average latency in ms of sending replicated mutations to remote cluster. Per-replication. (measured from wtavg_docs_latency)"
            },
            "wtavg_meta_latency": {
              unit: "millisecond",
              title: "XDCR Meta Batch Latency",
              desc: "Weighted average latency in ms of sending getMeta and waiting for a conflict solution result from remote cluster. Per-replication. (measured from wtavg_meta_latency)"
            }
          }
        }
      }
    }
  }
})();
