(function() {

  //
  // the qwConstantsService contains a number of constants used by the query workbench, such as
  // queries, URL prefixes, etc. This version is defined for the regular query workbench inside
  // the Couchbase admin UI, a different version will be defined for CBAS, and for the stand-alone
  // version
  //

  angular.module('qwQuery').factory('qwConstantsService', getQwConstantsService);

  getQwConstantsService.$inject = [];

  function getQwConstantsService() {

    var qwConstantsService = {};

    // do we automatically run queries if the user clicks enter after a semicolon?
    qwConstantsService.autoExecuteQueryOnEnter = true;

    // don't allow multiple queries to run at once
    qwConstantsService.forbidMultipleQueries = true;

    // URL to use for running queries
    qwConstantsService.queryURL = "../_p/query/query/service";

    // should we get passwords from the Couchbase server?
    qwConstantsService.getCouchbaseBucketPasswords = false;

    // should we run 'explain' in the background for each query?
    qwConstantsService.autoExplain = true;

    // should we show the bucket analysis pane at all?
    qwConstantsService.showBucketAnalysis = true;

    // allow a suffix to the key used for local storage
    qwConstantsService.localStorageSuffix = "";

    // query language mode for ACE editor
    qwConstantsService.queryMode = 'n1ql';

    // should queries include an array of credentials? ("creds")
    qwConstantsService.sendCreds = true;

    qwConstantsService.standAloneMode = true;

    // the following query asks Couchbase for a list of keyspaces, returning the 'id',
    // and a 'has_prim' boolean indicating whether or not it has a primary index, and
    // 'has_sec' indicating secondary indexes. For a different system, just make sure
    // the returned schema has 'id' and 'has_prim'.
    qwConstantsService.keyspaceQuery =
      "select max(keyspace_id) id, max(has_primary) has_prim, max(has_second) has_sec, max(secondary_indexes) sec_ind from (" +
      " select indexes.keyspace_id, true has_primary" +
      "  from system:indexes where is_primary = true and state = 'online'" +
      "  union" +
      "  select indexes.keyspace_id, true has_second, array_agg(indexes.index_key) secondary_indexes" +
      "  from system:indexes where state = 'online' and is_primary is missing or is_primary = false group by keyspace_id having keyspace_id is not null" +
      "  union" +
      "   select id keyspace_id from system:keyspaces except (select indexes.keyspace_id from system:indexes where state = 'online' union select \"\" keyspace_id)" +
      "  ) foo group by keyspace_id having keyspace_id is not null order by keyspace_id";

    // should we permit schema inquiries in the bucket analysis pane?
    qwConstantsService.showSchemas = true;

    // labels for different types of buckets in the analysis pane
    qwConstantsService.fullyQueryableBuckets = "Fully Queryable Buckets";
    qwConstantsService.queryOnIndexedBuckets = "Queryable on Indexed Fields";
    qwConstantsService.nonIndexedBuckets = "Non-Indexed Buckets";

    //
    // the nsserver proxy has a maximum request size
    qwConstantsService.maxRequestSize = 1048500;

    //
    // should we show the query options button?
    qwConstantsService.showOptions = true;

    //
    //
    // all done creating the service, now return it
    //

    return qwConstantsService;
  }



})();