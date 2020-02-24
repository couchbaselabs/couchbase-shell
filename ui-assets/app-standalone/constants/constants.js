(function () {
  "use strict";

  angular
    .module('app')
    .constant('bucketsFormConfiguration', {
      authType: 'sasl',
      name: '',
      conflictResolutionType: 'seqno',
      saslPassword: '',
      bucketType: 'membase',
      evictionPolicy: 'valueOnly',
      evictionPolicyEphemeral: 'noEviction',
      replicaNumber: "1",
      replicaIndex: "0",
      threadsNumber: "3",
      flushEnabled: "0",
      ramQuotaMB: "0",
      uri: '/pools/default/buckets',
      purgeInterval: "3",
      compressionMode: "passive",
      maxTTL: 0,
      storageBackend: 'couchstore',
    })
    .constant('daysOfWeek', [
      'Monday',
      'Tuesday',
      'Wednesday',
      'Thursday',
      'Friday',
      'Saturday',
      'Sunday'
    ])
    .constant('knownAlerts', [
      'auto_failover_node',
      'auto_failover_maximum_reached',
      'auto_failover_other_nodes_down',
      'auto_failover_cluster_too_small',
      'auto_failover_disabled',
      'ip',
      'disk',
      'overhead',
      'ep_oom_errors',
      'ep_item_commit_failed',
      'audit_dropped_events',
      'indexer_ram_max_usage',
      'ep_clock_cas_drift_threshold_exceeded',
      'communication_issue'
    ])
    .constant('timeUnitToSeconds', {
      minute: 60,
      hour: 3600,
      day: 86400,
      week: 691200,
      month: 2678400,
      year: 31622400
    })
    .constant("docsLimit", 1000)
    .constant("docBytesLimit", 256 * 1024)
    .constant("viewsPerPageLimit", 6)
    .constant("IEC", {
      Ki: 1024,
      Mi: 1048576,
      Gi: 1073741824
    });
})();
