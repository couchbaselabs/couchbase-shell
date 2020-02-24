(function() {

  angular.module('qwQuery').factory('qwQueryService', getQwQueryService);

  getQwQueryService.$inject = ['$rootScope','$q', '$uibModal', '$timeout', '$http', 'mnPendingQueryKeeper',
    'validateQueryService', 'qwConstantsService','qwQueryPlanService','mnPoolDefault',
    'mnPools','mnAuthService', 'mnServersService', 'qwFixLongNumberService'];

  function getQwQueryService($rootScope, $q, $uibModal, $timeout, $http, mnPendingQueryKeeper, validateQueryService,
      qwConstantsService,qwQueryPlanService,mnPoolDefault,mnPools,mnAuthService,
      mnServersService,qwFixLongNumberService) {

    var qwQueryService = {};

    //
    // remember which tab is selected for output style: JSON, table, or tree
    //

    qwQueryService.outputTab = 1;     // remember selected output tab
    qwQueryService.selectTab = function(newTab) {
      // some tabs are not available in some modes
      switch (newTab) {
      case 6: // advice is only available in EE
        if (!mnPools.export.isEnterprise)
          newTab = 1;
        break;
      }

      qwQueryService.outputTab = newTab;
     };
    qwQueryService.isSelected = function(checkTab) {return qwQueryService.outputTab === checkTab;};


    var monitoringOptions = {
        selectedTab: 1,
        autoUpdate: true,
        active_sort_by: 'elapsedTime',
        active_sort_reverse: true,
        completed_sort_by: 'elapsedTime',
        completed_sort_reverse: true,
        prepared_sort_by: 'elapsedTime',
        prepared_sort_reverse: true
    };
    qwQueryService.selectMonitoringTab = function(newTab) {monitoringOptions.selectedTab = newTab; saveStateToStorage();};
    qwQueryService.getMonitoringSelectedTab = function() {return monitoringOptions.selectedTab;};
    qwQueryService.isMonitoringSelected = function(checkTab) {return monitoringOptions.selectedTab === checkTab;};
    qwQueryService.getMonitoringAutoUpdate = function() {return monitoringOptions.autoUpdate;};
    qwQueryService.setMonitoringAutoUpdate = function(newValue) {monitoringOptions.autoUpdate = newValue; saveStateToStorage();};
    qwQueryService.getMonitoringOptions = function() {return monitoringOptions};

    // access to our most recent query result, and functions to traverse the history
    // of different results

    qwQueryService.getCurrentResult = getCurrentResult;
    qwQueryService.getCurrentIndexNumber = getCurrentIndexNumber;
    qwQueryService.getCurrentIndex = getCurrentIndex;
    qwQueryService.setCurrentIndex = setCurrentIndex;
    qwQueryService.clearHistory = clearHistory;
    qwQueryService.clearCurrentQuery = clearCurrentQuery;
    qwQueryService.hasPrevResult = hasPrevResult;
    qwQueryService.hasNextResult = hasNextResult;
    qwQueryService.prevResult = prevResult;
    qwQueryService.nextResult = nextResult;
    qwQueryService.addNewQueryAtEndOfHistory = addNewQueryAtEndOfHistory;
    qwQueryService.addSavedQueryAtEndOfHistory = addSavedQueryAtEndOfHistory;

    qwQueryService.canCreateBlankQuery = canCreateBlankQuery;

    qwQueryService.getPastQueries = function() {return(pastQueries);}
    qwQueryService.getQueryHistoryLength = function() {return(pastQueries.length);}

    qwQueryService.emptyResult = emptyResult;

    //
    // keep track of the bucket and field names we have seen, for use in autocompletion
    //

    qwQueryService.autoCompleteTokens = {}; // keep a map, name and kind
    qwQueryService.autoCompleteArray = [];  // array for use with Ace Editor

    // execute queries, and keep track of when we are busy doing so

    //qwQueryService.executingQuery = {busy: false};
    qwQueryService.currentQueryRequest = null;
    qwQueryService.currentQueryRequestID = null;
    qwQueryService.executeUserQuery = executeUserQuery;
    qwQueryService.cancelQuery = cancelQuery;
    qwQueryService.cancelQueryById = cancelQueryById;

    qwQueryService.executeQueryUtil = executeQueryUtil;

    qwQueryService.saveStateToStorage = saveStateToStorage;
    qwQueryService.loadStateFromStorage = loadStateFromStorage;
    qwQueryService.getQueryHistory = getQueryHistory;

    // update store the metadata about buckets

    qwQueryService.buckets = [];
    qwQueryService.bucket_names = [];
    qwQueryService.indexes = [];
    qwQueryService.updateBuckets = updateBuckets;             // get list of buckets
    qwQueryService.updateBucketCounts = updateBucketCounts;   // get list of buckets
    qwQueryService.getSchemaForBucket = getSchemaForBucket;   // get schema

    qwQueryService.runAdvise = runAdvise;
    qwQueryService.runAdviseOnLatest = runAdviseOnLatest;
    qwQueryService.showErrorDialog = showErrorDialog;
    qwQueryService.showWarningDialog = showWarningDialog;
    qwQueryService.hasRecommendedIndex = hasRecommendedIndex;

    qwQueryService.workbenchUserInterest = 'editor';

    mnPools.get().then(function (pools) {qwQueryService.pools = pools;});

//    mnAuthService.whoami().then(function (resp) {
//      if (resp) qwQueryService.user = resp;
//    });


    //
    // keep track of active queries, complete requests, and prepared statements
    //

    var active_requests = [];
    var completed_requests = [];
    var prepareds = [];

    var active_updated = "never"; // last update time
    var completed_updated = "never"; // last update time
    var prepareds_updated = "never"; // last update time

    qwQueryService.monitoring = {
        active_requests: active_requests,
        completed_requests: completed_requests,
        prepareds: prepareds,

        active_updated: active_updated,
        completed_updated: completed_updated,
        prepareds_updated: prepareds_updated,
    };

    qwQueryService.updateQueryMonitoring = updateQueryMonitoring;

    // for the front-end, distinguish error status and good statuses

    qwQueryService.status_success = status_success;
    qwQueryService.status_fail = status_fail;

    function status_success() {return(getCurrentResult().status_success());}
    function status_fail()    {return(getCurrentResult().status_fail());}

    //
    // here are some options we use while querying
    //

    qwQueryService.options = {
        timings: true,
        auto_infer: true,
        auto_format: false,
        dont_save_queries: false,
        max_parallelism: "",
        scan_consistency: "not_bounded",
        positional_parameters: [],
        named_parameters: [],
        query_timeout: 600
    };

    // clone options so we can have a scratch copy for the dialog box
    qwQueryService.clone_options = function() {
        return {
          timings: qwQueryService.options.timings,
          auto_infer: qwQueryService.options.auto_infer,
          auto_format: qwQueryService.options.auto_format,
          dont_save_queries: qwQueryService.options.dont_save_queries,
          max_parallelism: qwQueryService.options.max_parallelism,
          scan_consistency: qwQueryService.options.scan_consistency,
          positional_parameters: qwQueryService.options.positional_parameters.slice(0),
          named_parameters: qwQueryService.options.named_parameters.slice(0),
          query_timeout: qwQueryService.options.query_timeout
        };
    };

    //
    // a few variables for keeping track of the doc editor
    //

    qwQueryService.doc_editor_options = {
        selected_bucket: null,
        query_busy: false,
        show_tables: false,
        show_id: true, // show ID vs range of IDs
        limit: 10,
        offset: 0,
        where_clause: '',
        doc_id: '',
        doc_id_start: '',
        doc_id_end: '',
        current_query: '',
        current_result: []
    };

    qwQueryService.query_plan_options = {
        orientation: 1
    };

    //
    // this structure holds the current query text, the current query result,
    // and defines the object for holding the query history
    //

    function QueryResult(status,elapsedTime,executionTime,resultCount,resultSize,result,
        data,query,requestID,explainResult,mutationCount,warnings,sortCount,lastRun,status,advice) {
      this.status = status;
      this.resultCount = resultCount;
      this.mutationCount = mutationCount;
      this.resultSize = resultSize;
      this.result = result;
      this.data = data;
      this.query = query;
      this.requestID = requestID;
      this.explainResult = explainResult;
      if (explainResult)
        this.explainResultText = JSON.stringify(explainResult,null,'  ');
      else
        this.explainResultText = "";

      this.elapsedTime = truncateTime(elapsedTime);
      this.executionTime = truncateTime(executionTime);
      this.warnings = warnings;
      this.sortCount = sortCount;

      // when last run?
      this.lastRun = lastRun;
      this.status = status;

      // query advice
      this.advice = advice
    };

    // elapsed and execution time come back with ridiculous amounts of
    // precision, and some letters at the end indicating units.

    function truncateTime(timeStr)
    {
      var timeEx = /([0-9.]+)([a-z]+)/i; // number + time unit string

      if (timeStr && timeEx.test(timeStr)) {
        var parts = timeEx.exec(timeStr);
        var num = Number(parts[1]).toFixed(2); // truncate number part
        if (!isNaN(num))
          return(num + parts[2]);
      }

      return(timeStr); // couldn't match, just return orig value
    }


    QueryResult.prototype.clone = function()
    {
      return new QueryResult(this.status,this.elapsedTime,this.executionTime,this.resultCount,
          this.resultSize,this.result,this.data,this.query,this.requestID,this.explainResult,
          this.mutationCount,this.warnings,this.sortCount,this.lastRun,this.status,this.advice);
    };

    QueryResult.prototype.status_success = function() {
      return(this.status == 'success' || this.status == 'explain success');
    };
    QueryResult.prototype.status_fail = function()
    {return(this.status == '400' ||
        this.status == 'errors' ||
        this.status == '500' ||
        this.status == '404' ||
        this.status == 'stopped' ||
        this.status == 'explain error');
    };

    //
    // clone a query object, but omit the data and plan (which might take lots of space)
    //

    var un_run_status = "Not yet run";
    var un_run_query_data = {"No data to display": "Hit execute to run query."};
    var un_run_query_text =  JSON.stringify(un_run_query_data);

    QueryResult.prototype.clone_for_storage = function() {
      var res = new QueryResult(this.status,'','',this.resultCount,
          '',
          un_run_query_text,
          un_run_query_data,
          this.query,
          '',
          un_run_query_data,
          this.mutationCount,this.warnings,this.sortCount,this.lastRun,this.status);

      res.explainResultText = un_run_query_text;

      return res;
    }

    QueryResult.prototype.hasData = function() {
      return(this.result !== un_run_query_text);
    }

    QueryResult.prototype.copyIn = function(other)
    {
      this.status = other.status;
      this.elapsedTime = truncateTime(other.elapsedTime);
      this.executionTime = truncateTime(other.executionTime);
      this.resultCount = other.resultCount;
      this.mutationCount = other.mutationCount;
      this.resultSize = other.resultSize;
      this.result = other.result;
      this.data = other.data;
      this.query = other.query;
      this.requestID = other.requestID;
      this.explainResult = other.explainResult;
      this.explainResultText = other.explainResultText;
      this.warnings = other.warnings;
      this.sortCount = other.sortCount;
      if (_.isString(other.lastRun))
        this.lastRun = new Date(other.lastRun);
      else
        this.lastRun = other.lastRun;
      this.status = other.status;
      this.advice = other.advice;
    };

    //
    // how recently was the query run (if at all)?
    //

    QueryResult.prototype.getLastRun = function() {
      // need a lastRun time to see how long ago it was
      if (!this.lastRun || !_.isDate(this.lastRun))
        return(null);

      var howRecent = (new Date().getTime() - this.lastRun.getTime())/1000;

      // if the query is still running, output how long
      if (this.busy) {
        var recentStr = "for ";
        if (howRecent < 60)
          recentStr += "less than a minute.";
        else if (howRecent > 60)
          recentStr += Math.round(howRecent/60) + ' minutes';
        return recentStr;
      }

      // figure out how long ago it was
      var recentStr = '';
      if (howRecent < 60)
        recentStr += ' just now';
      else if (howRecent < 3600)
        recentStr += Math.round(howRecent/60) + ' min ago';
      else if (howRecent < 86400)
        recentStr += Math.round(howRecent/3600) + ' hrs ago';
      else
        recentStr += this.lastRun.toDateString(); //+ ' at ' + this.lastRun.getHours() + ':' + this.lastRun.getMinutes();

      return(recentStr);
    }

    QueryResult.prototype.getLastDetails = function () {
      var status = '';

      if (this.mutationCount)
        status += ', ' + this.mutationCount + ' mutations';
      else if (this.resultCount)
        status += ', ' + this.resultCount + ' documents';

      return(status);
    }

    //
    // structures for remembering queries and results
    //

    var dummyResult = new QueryResult('','','','','','',{},'');
    //var lastResult = dummyResult.clone();
    var savedResultTemplate = dummyResult.clone();
    savedResultTemplate.status = "";
    savedResultTemplate.result = un_run_query_text;
    savedResultTemplate.data = un_run_query_data;
    savedResultTemplate.explainResult = savedResultTemplate.data;
    savedResultTemplate.explainResultText = savedResultTemplate.result;

    var newQueryTemplate = dummyResult.clone();
    newQueryTemplate.status = un_run_status;
    newQueryTemplate.result = un_run_query_text;
    newQueryTemplate.data = un_run_query_data;

    var executingQueryTemplate = dummyResult.clone();
    executingQueryTemplate.status = "executing";
    executingQueryTemplate.result = '{"status": "Executing statement"}';
    executingQueryTemplate.data = {status: "Executing statement"};
    executingQueryTemplate.resultSize = 0;
    executingQueryTemplate.resultCount = 0;

    var pastQueries = [];       // keep a history of past queries and their results
    var currentQueryIndex = 0;  // where in the array are we? we start past the
                                // end of the array, since there's no history yet
    pastQueries.push(newQueryTemplate.clone()); // start off with a blank query

    function getCurrentResult() {
      // sanity checks to prevent MB-32954
      if (!pastQueries) pastQueries = [newQueryTemplate.clone()];
      if (currentQueryIndex < 0 || currentQueryIndex > pastQueries.length)
        currentQueryIndex = 0;
      if (!pastQueries[currentQueryIndex])
        pastQueries[currentQueryIndex] = newQueryTemplate.clone();
      return pastQueries[currentQueryIndex];
    }

    function emptyResult() {
        return(!pastQueries[currentQueryIndex] ||
            pastQueries[currentQueryIndex].result === savedResultTemplate.result);
    }

   //
    // where are we w.r.t. the query history?
    //

    function getCurrentIndex() {
      return (currentQueryIndex+1) + "/" + (pastQueries.length == 0 ? 1 : pastQueries.length);
    }

    function getCurrentIndexNumber() {
      return (currentQueryIndex);
    }

    function setCurrentIndex(index) {
      if (index < 0 || index >= pastQueries.length || index == currentQueryIndex)
        return;

      // if the current query was edited but not run, restore query to match results
      if (getCurrentResult().savedQuery && getCurrentResult().savedQuery != getCurrentResult().query)
        getCurrentResult().query = getCurrentResult().savedQuery;

      currentQueryIndex = index;

      // remember the current query in case the user edits it, then wants to revert
      getCurrentResult().savedQuery = getCurrentResult().query;
    }

    //
    // we want to store our state in the browser, if possible
    //

    function supportsHtml5Storage() {
      try {
        return 'localStorage' in window && window['localStorage'] !== null;
      } catch (e) {
        return false;
      }
    }

    var hasLocalStorage = supportsHtml5Storage();
    var localStorageKey = 'CouchbaseQueryWorkbenchState_' + window.location.host
    + qwConstantsService.localStorageSuffix;

    function loadStateFromStorage() {
      // make sure we have local storage

      //console.log("Trying to load from storage...");

      if (hasLocalStorage && _.isString(localStorage[localStorageKey])) try {
        var savedState = JSON.parse(localStorage[localStorageKey]);
        //console.log("Got saved state: " + JSON.stringify(savedState));
//        if (savedState.lastResult) {
//          //console.log("Got last result: " + JSON.stringify(savedState.lastResult));
//          lastResult.copyIn(savedState.lastResult);
//        }
//        else
//          console.log("No last result");

        if (savedState.pastQueries) {
          pastQueries = [];
          _.forEach(savedState.pastQueries,function(queryRes,index) {
            var newQuery = new QueryResult();
            newQuery.copyIn(queryRes);
            pastQueries.push(newQuery);
          });
        }

        // handle case of no queries in history
        if (pastQueries.length == 0)
          pastQueries.push(newQueryTemplate.clone());

        if (savedState.currentQueryIndex && savedState.currentQueryIndex < pastQueries.length)
          setCurrentIndex(savedState.currentQueryIndex);
        else
          setCurrentIndex(pastQueries.length - 1);

        getCurrentResult().savedQuery = getCurrentResult().query; // remember query if edited later

        if (savedState.outputTab)
          qwQueryService.selectTab(savedState.outputTab);
        if (savedState.options)
          qwQueryService.options = savedState.options;
        if (savedState.doc_editor_options) {
          if (!savedState.doc_editor_options.hasOwnProperty('show_tables'))
            savedState.doc_editor_options.show_tables = false;
          if (!savedState.doc_editor_options.hasOwnProperty('show_id'))
            savedState.doc_editor_options.show_id = true;
          qwQueryService.doc_editor_options = savedState.doc_editor_options;
        }
        if (savedState.query_plan_options) {
          qwQueryService.query_plan_options = savedState.query_plan_options;
        }

        if (savedState.monitoringOptions) {
          monitoringOptions = savedState.monitoringOptions;
          // handle backward compatibility
          if (!monitoringOptions.active_sort_by) {
            monitoringOptions.active_sort_by = 'elapsedTime';
            monitoringOptions.active_sort_reverse = true;
            monitoringOptions.completed_sort_by = 'elapsedTime';
            monitoringOptions.completed_sort_reverse = true;
            monitoringOptions.prepared_sort_by = 'elapsedTime';
            monitoringOptions.prepared_sort_reverse = true;
          }
        }

        // handle case where stored value of options might be not yet defined
        if (qwQueryService.options.auto_infer !== true && qwQueryService.options.auto_infer !== false)
          qwQueryService.options.auto_infer = true;

        if (qwQueryService.options.auto_format !== true && qwQueryService.options.auto_format !== false)
          qwQueryService.options.auto_format = false;

        if (qwQueryService.options.dont_save_queries !== true && qwQueryService.options.dont_save_queries !== false)
          qwQueryService.options.dont_save_queries = false;

      } catch (err) {console.log("Error loading state: " + err);}
    }


    function getQueryHistory(full) {
      // create a structure to hold the current state. To save state we will only
      // save queries, and not their results (which might well exceed the 5MB
      // we have available

      var savedState = {};
      savedState.pastQueries = [];
      savedState.outputTab = qwQueryService.outputTab;
      savedState.currentQueryIndex = currentQueryIndex;
      savedState.lastResult = getCurrentResult().clone_for_storage(); // for backward compatability
      savedState.options = qwQueryService.options;

      savedState.doc_editor_options = {
          selected_bucket: qwQueryService.doc_editor_options.selected_bucket,
          show_tables: qwQueryService.doc_editor_options.show_tables,
          show_id: qwQueryService.doc_editor_options.show_id,
          query_busy: false,
          limit: qwQueryService.doc_editor_options.limit,
          offset: qwQueryService.doc_editor_options.offset,
          where_clause: qwQueryService.doc_editor_options.where_clause,
          current_query: '',
          current_result: [] // don't want to save the results - they could be big
      };

      savedState.query_plan_options = {
          orientation: qwQueryService.query_plan_options.orientation
      };

      savedState.monitoringOptions = monitoringOptions;

      if (!qwQueryService.options.dont_save_queries) _.forEach(pastQueries,function(queryRes,index) {
        if (full)
          savedState.pastQueries.push(queryRes.clone());
        else
          savedState.pastQueries.push(queryRes.clone_for_storage());
      });

      return(JSON.stringify(savedState));
    }


    function saveStateToStorage() {
      // nop if we don't have local storage
      if (!hasLocalStorage)
        return;

      //console.log("saving state, len: " + JSON.stringify(savedState).length);

      // there is no cross browser means to determine how much local
      // storage space is available. When we get an exception, warn the user
      // and let them figure out what to do
      try {
        localStorage[localStorageKey] = getQueryHistory();
      } catch (e) {
        // if the save failed, notify the user
        showWarningDialog("Warning: Unable to save query history, browser local storage exhausted. You can still run queries, but they won't be saved for future sessions. Try removing large queries from history.")
      }
      //
      //console.log("Saving state to storage: ");
    }

    //
    // functions for adding new tokens and refreshing the token array
    //

    function addToken(token, type) {
      // see if the token needs to be quoted
      if (token.indexOf(' ') >= 0 || token.indexOf('-') >= 0 && !token.startsWith('`'))
        token = '`' + token + '`';

      // if the token isn't already there, add it
      if (!qwQueryService.autoCompleteTokens[token])
        qwQueryService.autoCompleteTokens[token] = type;

      // if the token is already known, but the type is new, add it to the list
      else if (qwQueryService.autoCompleteTokens[token].indexOf(type) == -1)
        qwQueryService.autoCompleteTokens[token] += ", " + type;
    };


    function refreshAutoCompleteArray() {
      qwQueryService.autoCompleteArray.length = 0;

      for (var key in qwQueryService.autoCompleteTokens) {
        //console.log("Got autoCompleteToken key: " + key);
        qwQueryService.autoCompleteArray.push(
            {caption:key,snippet:key,meta:qwQueryService.autoCompleteTokens[key]});
      }
    };


    //
    // go over a schema and recursively put all the field names in our name map
    //

    function getFieldNamesFromSchema(schema,prefix) {
      //console.log("Got schema: " + JSON.stringify(schema, null, 4));

      if (!prefix)
        prefix = '';

      for (var i=0; i< schema.length; i++)
        _.forEach(schema[i]['properties'], function(field, field_name) {
          //console.log("Adding field prefix: " + prefix + ', field: ' +  field_name);
          //console.log("  field[properties]: " + field['properties']);
          //console.log("  field[items]: " + field['items']);
          //if (field['items'])
          // console.log("    field[items].subtype: " + field['items'].subtype);

          addToken(prefix + field_name,"field");
          //if (prefix.length == 0 && !field_name.startsWith('`'))
          //  addToken('`' + field_name + '`',"field");

          // if the field has sub-properties, make a recursive call
          if (field['properties']) {
            getFieldNamesFromSchema([field],prefix + field_name + ".");
          }

          // if the field has 'items', it is an array, make recursive call with array type
          if (field['items'] && field['items'].subtype) {
            getFieldNamesFromSchema([field['items'].subtype],prefix + field_name + "[0].");
          }

          else if (_.isArray(field['items'])) for (var i=0;i<field['items'].length;i++)
            if (field['items'][i].subtype) {
              getFieldNamesFromSchema([field['items'][i].subtype],prefix + field_name + "[0].");
            }
        });
    }

    //
    // the UI can really only display a small number of fields in a schema, so truncate when necessary\
    //

    function truncateSchema(schema) {

      for (var i=0; i< schema.length; i++) {
        var fieldCount = 0;
        var flavor = schema[i];

        _.forEach(schema[i]['properties'], function(field, field_name) {
          if (++fieldCount > 250) {
            flavor.truncated = true;
            delete flavor['properties'][field_name];
            return true;
          }

          // if the field has sub-properties, make a recursive call
          if (field['properties']) {
            truncateSchema([field]);
          }

          // if the field has 'items', it is an array, make recursive call with array type
          if (field['items'] && field['items'].subtype) {
            truncateSchema([field['items'].subtype]);
          }

          else if (_.isArray(field['items'])) for (var i=0;i<field['items'].length;i++)
            if (field['items'][i].subtype) {
              truncateSchema([field['items'][i].subtype]);
            }
        });
      }

    }


    //
    // for error checking, it would be nice highlight when a specified field is not found
    // in a given schema
    //

    function isFieldNameInSchema(schema,fieldName) {
      // all schemas have the name "*"
      if (fieldName == "*")
        return true;
      // the field name might be a plain string, it might be suffixed with "[]", and it might
      // have a subfield expression starting with a "."
      var firstDot = fieldName.indexOf(".");
      var fieldPrefix = fieldName.substring(0,(firstDot >= 0 ? firstDot : fieldName.length));
      var fieldSuffix = (firstDot >= 0 ? fieldName.substring(firstDot+1) : "");
      var arrayIndex = fieldPrefix.indexOf("[");
      if (arrayIndex >= 0)
        fieldPrefix = fieldPrefix.substring(0,fieldPrefix.indexOf("["));

      //console.log("fieldPrefix: *" + fieldPrefix + "* suffix: *" + fieldSuffix + "*");

      for (var i=0; i< schema.length; i++) // for each flavor
        for (var field_name in schema[i]['properties']) {
          if (field_name == fieldPrefix) { // found a possible match
            //console.log("  got match");

            if (fieldSuffix.length == 0)  // no subfields? we're done, yay!
              return true;

            var field = schema[i]['properties'][field_name];

            //console.log("  looking for subproperties in field: " + JSON.stringify(field,null,2));
            // if we had an array expr, check each subtype's subfields against the array schema
            if (arrayIndex > -1 && _.isArray(field['items'])) {
              for (var arrType = 0; arrType < field['items'].length; arrType++)
                if (isFieldNameInSchema([field['items'][arrType].subtype],fieldSuffix))
                  return true;
            }

            else if (arrayIndex > -1 && field.items.subtype) {
              if (isFieldNameInSchema([field.items.subtype],fieldSuffix))
                return true;
            }

            // if we have a non-array, check the subschema
            else if (arrayIndex == -1 && field['properties'] &&
                isFieldNameInSchema([field],fieldSuffix))
              return true;
          }
        }

      // if we get this far without finding it, return false
      return false;
    }

    //
    // we also keep a history of executed queries and their results
    // we will permit forward and backward traversal of the history
    //

    var tempResult = "Processing";
    var tempData = {status: "processing"};

    //
    // we can create a blank query at the end of history if we're at the last slot, and
    // the query there has already been run
    //

    function canCreateBlankQuery() {
      return (currentQueryIndex >= 0 &&
          currentQueryIndex == pastQueries.length - 1 &&
          getCurrentResult().query.trim() === pastQueries[pastQueries.length-1].query.trim() &&
          getCurrentResult().status != newQueryTemplate.status);
    }
    function hasPrevResult() {return currentQueryIndex > 0;}

    // we can go forward if we're back in the history, or if we are at the end and
    // want to create a blank history element
    function hasNextResult() {
      return (currentQueryIndex < pastQueries.length-1) ||
      canCreateBlankQuery();
    }

    function prevResult()
    {
      if (currentQueryIndex > 0) // can't go earlier than the 1st
      {
        // if the current query was edited but not run, restore query to match results
        if (getCurrentResult().savedQuery && getCurrentResult().savedQuery != getCurrentResult().query)
          getCurrentResult().query = getCurrentResult().savedQuery;

        currentQueryIndex--;

        getCurrentResult().savedQuery = getCurrentResult().query;
      }
    }

    function nextResult()
    {
      if (currentQueryIndex < pastQueries.length -1) // can we go forward?
      {
        // if the current query was edited but not run, restore query to match results
        if (getCurrentResult().savedQuery && getCurrentResult().savedQuery != getCurrentResult().query)
          getCurrentResult().query = getCurrentResult().savedQuery;

        currentQueryIndex++;

        getCurrentResult().savedQuery = getCurrentResult().query;
      }

      // if the end query has been run, and is unedited, create a blank query
      else if (canCreateBlankQuery()) {
        addNewQueryAtEndOfHistory();
      }
    }

    function addNewQueryAtEndOfHistory(query) {
      // if the end of the history is a blank query, add it there.

      if (pastQueries.length > 0 && pastQueries[pastQueries.length -1].query.length == 0) {
        pastQueries[pastQueries.length -1].query = query;
      }

      // otherwise, add a new query at the end of history

      else {
        var newResult = newQueryTemplate.clone();
        if (query)
          newResult.query  = query;
        else
          newResult.query = "";
        pastQueries.push(newResult);
      }

      currentQueryIndex = pastQueries.length - 1;
    }

    function addSavedQueryAtEndOfHistory(query) {
      var newResult = new QueryResult(); // create the right object
      newResult.copyIn(query);

      // if the end of the history is a blank query, add it there.

      if (pastQueries.length > 0 && pastQueries[pastQueries.length -1].query.length == 0) {
        pastQueries[pastQueries.length -1].query = newResult;
      }

      // otherwise, add a new query at the end of history

      else {
        pastQueries.push(newResult);
      }

      currentQueryIndex = pastQueries.length - 1;
    }

    //
    // clear the entire query history
    //

    function clearHistory() {
      // don't clear the history if any queries are running
      for (i = 0; i < pastQueries.length; i++)
        if (pastQueries[i].busy)
          return;

      //lastResult.copyIn(dummyResult);
      pastQueries.length = 0;
      currentQueryIndex = 0;
      var newResult = newQueryTemplate.clone();
      pastQueries.push(newResult);

      saveStateToStorage(); // save current history
    }

    //
    // clear the specified query, or if none specified the current query
    //

    function clearCurrentQuery(index) {
      // don't clear the history if existing queries are already running
      if (qwQueryService.getCurrentResult().busy || pastQueries.length <= index)
        return;

      // did they specify an index to delete?
      var delIndex = (index || index === 0) ? index : currentQueryIndex;

      pastQueries.splice(delIndex,1);
      if (currentQueryIndex >= pastQueries.length)
        currentQueryIndex = pastQueries.length - 1;

      //if (currentQueryIndex >= 0)
        //lastResult.copyIn(pastQueries[currentQueryIndex]);
      // did they delete everything?
//      else {
//        //lastResult.copyIn(dummyResult);
//        pastQueries.length = 0;
//        currentQueryIndex = 0;
//      }

      // make sure we have at least one query
      if (pastQueries.length == 0) {
        var newResult = newQueryTemplate.clone();
        pastQueries.push(newResult);
      }

      saveStateToStorage(); // save current history
    }

    /**
     * Fast UUID generator, RFC4122 version 4 compliant.
     * @author Jeff Ward (jcward.com).
     * @license MIT license
     * @link http://stackoverflow.com/questions/105034/how-to-create-a-guid-uuid-in-javascript/21963136#21963136
     **/
    var UUID = (function() {
      var self = {};
      var lut = []; for (var i=0; i<256; i++) { lut[i] = (i<16?'0':'')+(i).toString(16); }
      self.generate = function() {
        var d0 = Math.random()*0xffffffff|0;
        var d1 = Math.random()*0xffffffff|0;
        var d2 = Math.random()*0xffffffff|0;
        var d3 = Math.random()*0xffffffff|0;
        return lut[d0&0xff]+lut[d0>>8&0xff]+lut[d0>>16&0xff]+lut[d0>>24&0xff]+'-'+
        lut[d1&0xff]+lut[d1>>8&0xff]+'-'+lut[d1>>16&0x0f|0x40]+lut[d1>>24&0xff]+'-'+
        lut[d2&0x3f|0x80]+lut[d2>>8&0xff]+'-'+lut[d2>>16&0xff]+lut[d2>>24&0xff]+
        lut[d3&0xff]+lut[d3>>8&0xff]+lut[d3>>16&0xff]+lut[d3>>24&0xff];
      }
      return self;
    })();

    //
    // cancelQuery - if a query is running, cancel it
    //

    function cancelQuery(queryResult) {
      // if this is a batch query, with multiple child queries, we need to find which one
      // is currently running, and cancel that.
      if (queryResult.batch_results)
        for (i=0; i<queryResult.batch_results.length; i++) if (queryResult.batch_results[i].busy) {
          queryResult = queryResult.batch_results[i]; // cancel this child
          break;
        }

      //console.log("Cancelling query, currentQuery: " + queryResult.client_context_id);
      if (queryResult && queryResult.client_context_id != null) {
        var queryInFly = mnPendingQueryKeeper.getQueryInFly(queryResult.client_context_id);
        queryInFly && queryInFly.canceler("test");

        //
        // also submit a new query to delete the running query on the server
        //

        var query = 'delete from system:active_requests where clientContextID = "' +
          queryResult.client_context_id + '";';

        executeQueryUtil(query,false)

        .then(function success() {
//        console.log("Success cancelling query.");
        },

        // sanity check - if there was an error put a message in the console.
        function error(resp) {
          logWorkbenchError("Error cancelling query: " + JSON.stringify(resp));
//          console.log("Error cancelling query.");
        });

      }
    }


    //
    // query monitoring might want to cancel queries this way

    function cancelQueryById(requestId) {
      //console.log("Cancelling query: " + requestId);
      var query = 'delete from system:active_requests where requestId = "' +
        requestId + '";';

      executeQueryUtil(query,false)

        .then(function success() {
//        console.log("Success cancelling query.");
        },

      // sanity check - if there was an error put a message in the console.
      function error(resp) {
          logWorkbenchError("Error cancelling query: " + JSON.stringify(resp));
//        var data = resp.data, status = resp.status;
//        console.log("Error cancelling query: " + query);
      });
    }

    //
    // we run queries many places, the following function calls $http to run
    // the query, and returns the promise so the caller can handle success/failure callbacks.
    // queryText - the query to run
    // is_user_query - with user queries, we need to
    //   1) set a client_context_id to allow the query to be cancelled
    //   2) transform responses to handle ints > 53 bits long?
    //   3) set qwQueryService.currentQueryRequestID and qwQueryRequest.currentQueryRequest
    //

    function executeQueryUtil(queryText, is_user_query) {
      //console.log("Running query: " + queryText);
      var request = buildQueryRequest(queryText,is_user_query);

      // if the request can't be built because the query is too big, return a dummy
      // promise that resolves immediately. This needs to follow the angular $http
      // promise, which supports .success and .error as well as .then

      if (!request) {
        var dummy = Promise.resolve({errors: "Query too long"});
        //dummy.success = function(fn) {/*nop*/ return(dummy);};
        //dummy.error = function(fn) {dummy.then(fn); return(dummy);};
        dummy.origThen = dummy.then;
        dummy.then = function(fn1,fn2) {dummy.origThen(fn1,fn2); return(dummy);};
        return(dummy);
      }

      return($http(request));
    }

    function logWorkbenchError(errorText) {
      $http({
          url: "/logClientError",
          method: "POST",
          data: errorText,
      });
    }

    function buildQueryRequest(queryText, is_user_query, queryOptions) {

      //console.log("Building query: " + queryText);
      //
      // create a data structure for holding the query, and the credentials for any SASL
      // protected buckets
      //

      if (!_.isNumber(qwQueryService.options.query_timeout) ||
          qwQueryService.options.query_timeout == 0)
          qwQueryService.options.query_timeout = 600;

      var queryData = {statement: queryText, pretty: false, timeout: (qwQueryService.options.query_timeout + 's')};

      // are there options we need to add to the query request?

      if (queryOptions) {
        if (queryOptions.timings) // keep track of timings for each op?
          queryData.profile = "timings";

        if (queryOptions.max_parallelism && queryOptions.max_parallelism.length > 0)
          queryData.max_parallelism = queryOptions.max_parallelism;

        if (queryOptions.scan_consistency)
          queryData.scan_consistency = queryOptions.scan_consistency;

        // named and positional parameters
        if (queryOptions.positional_parameters && queryOptions.positional_parameters.length > 0)
          queryData.args = queryOptions.positional_parameters;

        if (queryOptions.named_parameters)
          for (var i=0; i < queryOptions.named_parameters.length; i++)
            queryData[queryOptions.named_parameters[i].name] = queryOptions.named_parameters[i].value;

        //console.log("Running query: " + JSON.stringify(queryData));
      }

      // if the user might want to cancel it, give it an ID

      if (is_user_query) {
        queryData.client_context_id = UUID.generate();
        queryData.pretty = true;
      }
      // for auditing, note that the non-user queries are "INTERNAL"
      else
        queryData.client_context_id = "INTERNAL-" + UUID.generate();

      //console.log("Got context: " + queryData.client_context_id + ", query: " + queryText);

      //
      // build the query request
      //

      var queryRequest;
      var userAgent = 'Couchbase Query Workbench';
      if (mnPoolDefault.export.thisNode && mnPoolDefault.export.thisNode.version)
        userAgent += ' (' + mnPoolDefault.export.thisNode.version + ')';

      var queryRequest = {
        url: qwConstantsService.queryURL,
        method: "POST",
        headers: {'Content-Type':'application/json','ns-server-proxy-timeout':
                  (qwQueryService.options.query_timeout+1)*1000,
                  'ignore-401':'true','CB-User-Agent': userAgent},
        data: queryData,
        mnHttp: {
          isNotForm: true,
          group: "global"
        }
      };

      // if it's a userQuery, make sure to handle really long ints, and remember the
      // request in case we need to cancel

      if (is_user_query) {
        queryRequest.transformResponse = qwFixLongNumberService.fixLongInts;
        qwQueryService.currentQueryRequest = queryRequest;
      }

      //
      // check the queryRequest to make sure it's not too big
      //

      if (qwConstantsService.maxRequestSize &&
          JSON.stringify(queryRequest).length >= qwConstantsService.maxRequestSize) {
        showErrorDialog("Query too large for GUI, try using CLI or REST API directly.")
        return(null);
      }

      //console.log("Built query: " + JSON.stringify(queryRequest));
      return(queryRequest);
    }

    //
    // convenience function to see if fields mentioned in a query are not found in the schema
    // for the buckets involved
    //

    function getProblemFields(fields) {
      var problem_fields = [];

      for (var f in fields) {
        var firstDot = f.indexOf(".");
        var bucketName = f.substring(0,firstDot);
        var fieldName = f.substring(firstDot + 1);
        //console.log("Checking field: " + f + ", bucket: " + bucketName);
        var bucket = _.find(qwQueryService.buckets,function (b) {return(b.id == bucketName);});
        if (bucket) {
          if (bucket && bucket.schema.length > 0 && !isFieldNameInSchema(bucket.schema,fieldName)) {
            problem_fields.push({field: fieldName, bucket: bucket.id});
            //console.log("Field: " + fieldName + " is not o.k.");
            //console.log("  Got bucket schema: " + JSON.stringify(bucket.schema,null,2));
          }
        }
      }

      return(problem_fields);
    }

    //
    // executeUserQuery
    //
    // take a query from the user, or possibly a string containing multiple queries separated by semicolons,
    // and execute them, updating the UI to show progress
    //

    function executeUserQuery(explainOnly) {
      // if the user edited an already-run query, add the edited query to the end of the history
      var query = getCurrentResult();
      if (query.savedQuery && query.savedQuery != query.query && query.lastRun) {
        var result = executingQueryTemplate.clone();
        result.query = query.query.trim();
        pastQueries.push(result);
        currentQueryIndex = pastQueries.length - 1; // after run, set current result to end
        query.query = query.savedQuery; // restore historical query to original value
      }

      // make sure that the current query isn't already running
      if (getCurrentResult().busy)
        return;

      getCurrentResult().busy = true;

      // clear any previous results, remember when we started
      var queryText = getCurrentResult().query;
      var newResult = getCurrentResult();
      newResult.copyIn(executingQueryTemplate);
      newResult.query = queryText;
      newResult.savedQuery = queryText;
      newResult.lastRun = new Date();

      // if we have multiple queries, pull them apart into an array so we can run them
      // in sequence
      var queries = [];
      var curQuery = '';
      var findSemicolons = /("(?:[^"\\]|\\.)*")|('(?:[^'\\]|\\.)*')|(\/\*(?:.|[\n\r])*\*\/)|(`(?:[^`]|``)*`)|((?:[^;"'`\/]|\/(?!\*))+)|(;)/g;
      var matchArray = findSemicolons.exec(queryText);

      while (matchArray != null) {
        //console.log("Got matchArray: " + JSON.stringify(matchArray));
        curQuery += matchArray[0];
        if (matchArray[0] == ';') {
          queries.push(curQuery);
          curQuery = '';
        }
        matchArray = findSemicolons.exec(queryText);
      }

      if (curQuery.length > 0)
        queries.push(curQuery); // get final query if unterminated with ;

      //console.log("Got queries: " + JSON.stringify(queries));

      // if we have a single query, run it. If we have multiple queries, run each one in sequence,
      // stopping if we see an error by chaining the promises together
      var queryExecutionPromise;

      if (queries.length > 1) {
        newResult.batch_results = [];
        newResult.busy = true;

        for (var i = 0; i < queries.length; i++)
          newResult.batch_results.push(newQueryTemplate.clone());

        newResult.explainResult = "Graphical plans not available for multiple query sequences.";
        queryExecutionPromise = runBatchQuery(newResult, queries, 0, explainOnly);
      }

      // otherwise only a single query, run it
      else
        queryExecutionPromise = executeSingleQuery(queryText,explainOnly,newResult)
        .then(
          function success() {
            if (!newResult.status_success()) // if errors, go to tab 1
              qwQueryService.selectTab(1);
          },
          function error() {qwQueryService.selectTab(1);}) // error, go to tab 1
          // when done, save the current state
          .finally(function() {saveStateToStorage(); /*finishQuery(newResult);*/});

      return(queryExecutionPromise);
    }

    //
    // recursive function to run queries one after the other
    //

    function runBatchQuery(parentResult, queryArray, curIndex, explainOnly) {

      // if we successfully executed the final query, set the parent status to the status of the last query
      if (curIndex >= queryArray.length) {
        finishParentQuery(parentResult,parentResult.batch_results.length - 1, false);
        return(Promise.resolve); // success!
      }

      // launch a query
      parentResult.status = "Executing " + (curIndex+1) + "/" + queryArray.length;

      return executeSingleQuery(queryArray[curIndex],explainOnly,parentResult.batch_results[curIndex]).then(
          function success() {
            addBatchResultsToParent(parentResult, curIndex);

            // only run the next query if this query was a success
            if (parentResult.batch_results[curIndex].status_success()) {
              runBatchQuery(parentResult, queryArray, curIndex+1, explainOnly);
            }
            // with failure, end the query
            else
              finishParentQuery(parentResult, curIndex, true);
          },
          // if we get failure, the parent status is the status of the last query to run
          function fail() {
            addBatchResultsToParent(parentResult, curIndex);
            finishParentQuery(parentResult, curIndex, true);
          }
      );
    }

    //
    // each time a child query finishes, add their results to the parent
    //

    function addBatchResultsToParent(parentResult, childIndex) {
      // is the parent set up for data yet?
      if (parentResult.result == executingQueryTemplate.result) {
        parentResult.data = [];
        parentResult.explainResult = [];
        parentResult.result = '';
        parentResult.explainResultText = '';
      }
      // otherwise we need to create a new parent result array so that the
      // directives doing $watch notice the change
      else {
        var newData = parentResult.data.slice();
        parentResult.data = newData;
      }

      // add the latest result
      parentResult.data.push({
          _sequence_num: childIndex + 1,
          _sequence_query: parentResult.batch_results[childIndex].query,
          _sequence_query_status: parentResult.batch_results[childIndex].status,
          _sequence_result: parentResult.batch_results[childIndex].data}
      );
      parentResult.explainResult.push({
        _sequence_num: childIndex + 1,
        _sequence_query: parentResult.batch_results[childIndex].query,
        _sequence_result: parentResult.batch_results[childIndex].explainResult});

      parentResult.result = JSON.stringify(parentResult.data, null, '  ');
      parentResult.explainResultText = JSON.stringify(parentResult.explainResult, null, '  ');
    }

    //
    // when the children are done, finish the parent
    //

    function finishParentQuery(parentResult, index, isError) {
      // the parent gets its status from the last query to run
      parentResult.status = parentResult.batch_results[index].status;

      // mark the parent as done, if errors were seen select tab 1, then save the history
      finishQuery(parentResult);
      if (isError)
        qwQueryService.selectTab(1);
      saveStateToStorage();
    }

    //
    // time values in metrics can get really ugly, e.g. 1.23423423432s or 334.993843ms
    //
    // let's round them
    //

    function simplifyTimeValue(timeValue) {
      var durationExpr = /(\d+m)?(?:(\d+\.\d+)s)?(?:(\d+\.\d+)ms)?(?:(\d+\.\d+)µs)?/;
      var result = '';

      var m = timeValue.match(durationExpr);

      if (m[1]) result += m[1];
      if (m[2]) {
        var seconds = Math.round(parseFloat(m[2])*10)/10;
        result += seconds + 's';
      }
      if (m[3]) {
        var ms = Math.round(parseFloat(m[3])*10)/10;
        result += ms + 'ms';
      }
      if (m[4]) {
        var us = Math.round(parseFloat(m[4])*10)/10;
        result += us + 'µs';
      }
      return(result)
    }

    //
    // when we have a single query, associated with a query result in the history,
    // we have a specific process to execute it:
    //
    // - mark the query as busy - we don't want to execute multiple times
    // - should we run EXPLAIN separately? If so, do it, update the query result
    // - should we run the plain query? If so, do it, update the query result
    // - return a promise resolving when either/both of those queries completes,
    //   marking the query as no longer busy when that happens
    //

    function executeSingleQuery(queryText,explainOnly,newResult) {
      var pre_post_ms = new Date().getTime(); // when did we start?
      var promises = []; // we may run explain only, or explain + actual  query
      //console.log("Running query: " + queryText);

      // make sure the result is marked as executing
      newResult.copyIn(executingQueryTemplate);
      newResult.query = queryText;
      newResult.busy = true;
      newResult.savedQuery = queryText;
      newResult.lastRun = new Date();

      //
      // if the query is not already an explain, run a version with explain to get the query plan,
      // unless the query is prepare - we can't explain those
      //

      var queryIsExplain = /^\s*explain/gmi.test(queryText);
      var queryIsPrepare = /^\s*prepare/gmi.test(queryText);
      var queryIsAdvise  = /^\s*advise/gmi.test(queryText);
      var explain_promise, advise_promise;

      // the result tabs can show data, explain results, or show advice. Make sure the tab setting is
      // appropriate for the query type
      switch (qwQueryService.outputTab) {
      case 1: // JSON
      case 2: // Table
      case 3: // Tree
        if (explainOnly)
          qwQueryService.selectTab(4); // vis for EE, text for CE
        else if (queryIsAdvise)
          qwQueryService.selectTab(6);
        // otherwise don't change it
        break;
      case 4: // visual plan
      case 5: // plan text
        if (!queryIsExplain && !explainOnly && !queryIsAdvise)
          qwQueryService.selectTab(1);
        else if (queryIsAdvise)
          qwQueryService.selectTab(6);
        break;
      case 6: // advice tab
        if (!queryIsExplain && !explainOnly && !queryIsAdvise)
          qwQueryService.selectTab(1);
        else if (queryIsExplain || explainOnly)
          qwQueryService.selectTab(4); // vis for EE, text for CE
        break;
      }

      //
      // run the explain version of the query, if appropriate
      //

      if (!queryIsExplain && (explainOnly || (qwConstantsService.autoExplain && !queryIsPrepare))) {

        var explain_request = buildQueryRequest("explain " + queryText, false, qwQueryService.options);
        if (!explain_request) {
          newResult.result = '{"status": "query failed"}';
          newResult.data = {status: "Query Failed."};
          newResult.status = "errors";
          newResult.resultCount = 0;
          newResult.resultSize = 0;

          // can't recover from error, finish query
          finishQuery(newResult);
          return(Promise.reject("building query failed"));
        }
        explain_promise = $http(explain_request)
        .then(function success(resp) {
          var data = resp.data, status = resp.status;
          //
          //console.log("explain success: " + JSON.stringify(data));

          // if the query finished first and produced a plan, ignore
          // the results of the 'explain'. Only proceed if no explainResult

          if (!newResult.explainResult) {
            // now check the status of what came back
            if (data && data.status == "success" && data.results && data.results.length > 0) try {

              // if we aren't running a regular query, set the status for explain-only
              if (!((queryIsExplain && explainOnly) || !explainOnly))
                newResult.status = "explain success";

              if (data.metrics && newResult.elapsedTime != '') {
                newResult.elapsedTime = simplifyTimeValue(data.metrics.elapsedTime);
                newResult.executionTime = simplifyTimeValue(data.metrics.executionTime);
                newResult.resultCount = data.metrics.resultCount;
                newResult.mutationCount = data.metrics.mutationCount;
                newResult.resultSize = data.metrics.resultSize;
                newResult.sortCount = data.metrics.sortCount;
              }

              var lists = qwQueryPlanService.analyzePlan(data.results[0].plan,null);
              newResult.explainResultText = JSON.stringify(data.results[0].plan, null, '    ');
              newResult.explainResult =
              {explain: data.results[0],
                  analysis: lists,
                  plan_nodes: qwQueryPlanService.convertN1QLPlanToPlanNodes(data.results[0].plan, null, lists)
              };

              if (_.isArray(lists.warnings) && lists.warnings.length > 0)
                newResult.warnings = JSON.stringify(lists.warnings);

              // let's check all the fields to make sure they are all valid
              var problem_fields = getProblemFields(newResult.explainResult.analysis.fields);
              if (problem_fields.length > 0)
                newResult.explainResult.problem_fields = problem_fields;
            }
            // need to handle any exceptions that might occur
            catch (exception) {
              console.log("Exception analyzing query plan: " + exception);
              newResult.explainResult = "Internal error generating query plan: " + exception;
              newResult.explainResultText = "Internal error generating query plan: " + exception;
              newResult.status = "explain error";
            }

            // if status != success
            else if (data.errors) {
              newResult.explainResult = data.errors;
              newResult.explainResult.query = explain_request.data.statement;
              newResult.explainResultText = JSON.stringify(data.errors, null, '    ');
              newResult.status = "explain error";
            }
            else {
              newResult.explainResult = {'error': 'No server response for explain.'};
              newResult.explainResultText = JSON.stringify(newResult.explainResult, null, '    ');
              newResult.status = "explain error";
            }
          }
        },
        /* error response from $http */
        function error(resp) {
          var data = resp.data, status = resp.status;
          //console.log("Explain error Data: " + JSON.stringify(data));
          //console.log("Explain error Status: " + JSON.stringify(status));

          // if we aren't running a regular query, set the status for explain-only
          if (!((queryIsExplain && explainOnly) || !explainOnly))
            newResult.status = status || "explain error";

          // we only want to pay attention to the result if the query hasn't finished
          // already and generated a more definitive query plan

          if (!newResult.explainResult) {

            if (data && _.isString(data)) {
              newResult.explainResult = {errors: data};
              newResult.explainResult.query_from_user = explain_request.data.statement;
            }
            else if (data && data.errors) {
              if (data.errors.length > 0)
                data.errors[0].query_from_user = explain_request.data.statement;
              newResult.explainResult = {errors: data.errors};
            }
            else {
              newResult.explainResult = {errors: "Unknown error getting explain plan"};
              newResult.explainResult.query_from_user = explain_request.data.statement;
            }

            newResult.explainResultText = JSON.stringify(newResult.explainResult, null, '  ');

            // if the query hasn't returned metrics, include the explain metrics,
            // so they know how long it took before the error

            if (data.metrics && newResult.elapsedTime != '') {
              newResult.elapsedTime = simplifyTimeValue(data.metrics.elapsedTime);
              newResult.executionTime = simplifyTimeValue(data.metrics.executionTime);
              newResult.resultCount = data.metrics.resultCount;
              newResult.mutationCount = data.metrics.mutationCount;
              newResult.resultSize = data.metrics.resultSize;
              newResult.sortCount = data.metrics.sortCount;
            }
          }

          return(Promise.resolve()); // don't want to short circuit resolution of other promises
        });

        promises.push(explain_promise);
      }

      //
      // Run the query as typed by the user?
      // - Above, we might have run it with "EXPLAIN" added to the beginning.
      // - If the user clicked "Explain", we only run the query if it already has "EXPLAIN" in the text
      // - If the user clicked "Execute", go ahead and run the query no matter how it looks.
      //

      if ((queryIsExplain && explainOnly) || !explainOnly) {

        var request = buildQueryRequest(queryText, true, qwQueryService.options);
        newResult.client_context_id = request.data.client_context_id;
        //console.log("Got client context id: " + newResult.client_context_id);

        if (!request) {
          newResult.result = '{"status": "Query Failed."}';
          newResult.data = {status: "Query Failed."};
          newResult.status = "errors";
          newResult.resultCount = 0;
          newResult.resultSize = 0;

          // make sure to only finish if the explain query is also done
          return(Promise.reject("building explain query failed"));
        }
        var query_promise = $http(request)
        // SUCCESS!
        .then(function success(resp) {
          var data = resp.data, status = resp.status;
//          console.log("Success for query: " + queryText);
//        console.log("Success Data: " + JSON.stringify(data));
//        console.log("Success Status: " + JSON.stringify(status));

          // Even though we got a successful HTTP response, it might contain warnings or errors
          // We need to be able to show both errors and partial results, or if there are no results
          // just the errors

          var result; // hold the result, or a combination of errors and result
          var isEmptyResult = (!_.isArray(data.results) || data.results.length == 0);

          // empty result, fill it with any errors or warnings
          if (isEmptyResult) {
            if (data.errors)
              result = data.errors;
            else if (data.warnings)
              result = data.warnings;

            // otherwise show some context, make it obvious that results are empty
            else {
              result = {};
              result.results = data.results;
            }
          }
          // non-empty result: use it
          else
            result = data.results;

          // if we have results, but also errors, record them in the result's warning object
          if (data.warnings && data.errors)
            newResult.warnings = "'" + JSON.stringify(data.warnings,null,2) + JSON.stringify(data.errors,null,2) + "'";
          else if (data.warnings)
            newResult.warnings = "'" + JSON.stringify(data.warnings,null,2) + "'";
          else if (data.errors)
            newResult.warnings = "'" + JSON.stringify(data.errors,null,2) + "'";
          if (data.status == "stopped") {
            result = {status: "Query stopped on server."};
          }

          if (_.isString(newResult.warnings))
            newResult.warnings = newResult.warnings.replace(/\n/g,'<br>').replace(/ /g,'&nbsp;');

          // if we got no metrics, create a dummy version
          if (!data.metrics) {
            data.metrics = {elapsedTime: 0.0, executionTime: 0.0, resultCount: 0, resultSize: "0", elapsedTime: 0.0}
          }

          newResult.status = data.status;
          newResult.elapsedTime = simplifyTimeValue(data.metrics.elapsedTime);
          newResult.executionTime = simplifyTimeValue(data.metrics.executionTime);
          newResult.resultCount = data.metrics.resultCount;
          if (data.metrics.mutationCount)
            newResult.mutationCount = data.metrics.mutationCount;
          newResult.resultSize = data.metrics.resultSize;
          newResult.sortCount = data.metrics.sortCount;
          if (data.rawJSON)
            newResult.result = data.rawJSON;
          else
            newResult.result = angular.toJson(result, true);
          newResult.data = result;
          newResult.requestID = data.requestID;

          // did we get query timings in the result? If so, update the plan

          if (data.profile && data.profile.executionTimings) try {
            var lists = qwQueryPlanService.analyzePlan(data.profile.executionTimings,null);
            newResult.explainResult =
            {explain: data.profile.executionTimings,
                analysis: lists,
                plan_nodes: qwQueryPlanService.convertN1QLPlanToPlanNodes(data.profile.executionTimings,null,lists)};
            newResult.explainResultText = JSON.stringify(newResult.explainResult.explain,null,'  ');

            // let's check all the fields to make sure they are all valid
            var problem_fields = getProblemFields(newResult.explainResult.analysis.fields);
            if (problem_fields.length > 0)
              newResult.explainResult.problem_fields = problem_fields;
          }

          // need to handle any exceptions that might occur
          catch (exception) {
            console.log("Exception analyzing query plan: " + exception);
            newResult.explainResult = "Internal error generating query plan: " + exception;
          }

          // if this was an explain query, analyze the results to get us a graphical plan

          if (queryIsExplain && data.results && data.results[0] && data.results[0].plan) try {
            var lists = qwQueryPlanService.analyzePlan(data.results[0].plan,null);
            newResult.explainResult =
            {explain: data.results[0],
                analysis: lists,
                plan_nodes: qwQueryPlanService.convertN1QLPlanToPlanNodes(data.results[0].plan,null,lists)
                /*,
              buckets: qwQueryService.buckets,
              tokens: qwQueryService.autoCompleteTokens*/};
            newResult.explainResultText = JSON.stringify(newResult.explainResult.explain, null, '  ');
          }
          // need to handle any exceptions that might occur
          catch (exception) {
            console.log("Exception analyzing query plan: " + exception);
            newResult.explainResult = "Internal error generating query plan: " + exception;
            //newResult.explainResultText = "Internal error generating query plan: " + exception;
          }

          // if the query was "advice select...", make sure the result gets put into advice

          if (queryIsAdvise) {
            if (data && data.status == "success" && data.results && data.results.length > 0)
              newResult.advice = data.results[0].advice.adviseinfo;
            else
              newResult.advice = newResult.result; // get the error message
          }


        },
        /* error response from $http */
        function error(resp) {
          var data = resp.data, status = resp.status;
//        console.log("Error resp: " + JSON.stringify(resp));
//        console.log("Error Data: " + JSON.stringify(data));

          // if we don't get query metrics, estimate elapsed time
          if (!data || !data.metrics) {
            var post_ms = new Date().getTime();
            newResult.elapsedTime = (post_ms - pre_post_ms) + "ms";
            newResult.executionTime = newResult.elapsedTime;
          }

          // no result at all? failure
          if (data === undefined) {
            newResult.result = '{"status": "Failure contacting server."}';
            newResult.data = {status: "Failure contacting server."};
            newResult.status = "errors";
            newResult.resultCount = 0;
            newResult.resultSize = 0;
            return;
          }

          // data is null? query interrupted
          if (data === null) {
            newResult.result = '{"status": "Query interrupted."}';
            newResult.data = {status: "Query interrupted."};
            newResult.status = "errors";
            newResult.resultCount = 0;
            newResult.resultSize = 0;
            return;
          }

          // result is a string? it must be an error message
          if (_.isString(data)) {
            newResult.data = {status: data};
            if (status && status == 504) {
              newResult.data.status_detail =
                "The query workbench only supports queries running for " + qwQueryService.options.query_timeout +
                " seconds. This value can be changed in the preferences dialog. You can also use cbq from the " +
                "command-line for longer running queries. " +
                "Certain DML queries, such as index creation, will continue in the " +
                "background despite the user interface timeout.";
            }

            newResult.result = JSON.stringify(newResult.data,null,'  ');
            newResult.status = "errors";
            return;
          }

          if (data.errors) {
            if (_.isArray(data.errors) && data.errors.length >= 1)
              data.errors[0].query = queryText;
            newResult.data = data.errors;
            newResult.result = JSON.stringify(data.errors,null,'  ');
          }

          if (status)
            newResult.status = status;
          else
            newResult.status = "errors";

          if (data.metrics) {
            newResult.elapsedTime = simplifyTimeValue(data.metrics.elapsedTime);
            newResult.executionTime = simplifyTimeValue(data.metrics.executionTime);
            newResult.resultCount = data.metrics.resultCount;
            if (data.metrics.mutationCount)
              newResult.mutationCount = data.metrics.mutationCount;
            newResult.resultSize = data.metrics.resultSize;
            newResult.sortCount = data.metrics.sortCount;
          }

          if (data.requestID)
            newResult.requestID = data.requestID;

          // make sure to only finish if the explain query is also done
          if (newResult.explainDone) {
            // when we have errors, don't show the plan tabs
            if (qwQueryService.isSelected(4) || qwQueryService.isSelected(5))
              qwQueryService.selectTab(1);
          }

          return(Promise.resolve()); // don't want to short circuit resolution of other promises
        });

        promises.push(query_promise);
      }

      //
      // let's run ADVISE on the query to see if there's a better way to do it
      //

      if (!explainOnly && !queryIsAdvise) {
        var advise_promise = runAdvise(queryText,newResult);
        if (advise_promise != null)
          promises.push(advise_promise);
      }

      // return a promise wrapping the one or two promises
      // when the queries are done, call finishQuery

      return($q.all(promises).then(
          function() {finishQuery(newResult);},
          function() {finishQuery(newResult);}
          ));
    }

    //
    // run ADVISE for a given query and queryResult, without also running the query or explain
    //

    function runAdviseOnLatest() {
      var query = getCurrentResult();
      var queryIsAdvise  = /^\s*advise/gmi.test(query.query);

      // if the query already starts with 'advise', run it as a regular query
      if (queryIsAdvise) {
        executeUserQuery(false);
        qwQueryService.selectTab(1);
        return;
      }

      // if the user edited an already-run query, add the edited query to the end of the history
      if (query.savedQuery && query.savedQuery != query.query && query.lastRun) {
        var result = executingQueryTemplate.clone();
        result.query = query.query.trim();
        pastQueries.push(result);
        currentQueryIndex = pastQueries.length - 1; // after run, set current result to end
        query.query = query.savedQuery; // restore historical query to original value
        query = getCurrentResult();
        saveStateToStorage();
      }

      var initialAdvice = "Getting advice for current query...";
      query.advice = initialAdvice;
      query.result = query.advice;
      query.data = {status: query.result};
      query.warnings = null;
      qwQueryService.selectTab(6);
      runAdvise(getCurrentResult().query,getCurrentResult()).then(
          function success(resp) {
            if (query.advice == initialAdvice)
              query.data = {adviseResult: resp};
            else
              query.data = {adviseResult: query.advice};

            query.result = JSON.stringify(query.data,null,2);

            if (_.isString(query.advice))
              query.status = "error";
            else
              query.status = "success";

            finishQuery(query);
          },
          function err(resp) {
              query.advice = 'Query not advisable';
              query.result = "Error getting advice."
              query.data = {adviseResult: query.result};
              query.status = "advise error";
              finishQuery(query);
          });
    };

    function runAdvise(queryText,queryResult) {
      queryResult.lastRun = new Date();

      var queryIsAdvisable = qwQueryService.pools && qwQueryService.pools &&
        /^\s*select|merge|update|delete/gmi.test(queryText);

      if (queryIsAdvisable && !multipleQueries(queryText)) {
        var advise_request = buildQueryRequest("advise " + queryText, false, qwQueryService.options);
        // log errors but ignore them
        if (!advise_request) {
          console.log("Couldn't build Advise query. ");
          return(Promise.resolve("building advise query failed"));
        }
        advise_promise = $http(advise_request)
        .then(function success(resp) {
          var data = resp.data, status = resp.status;
          //
          // if the query finished first and produced a plan, ignore
          // the results of the 'explain'. Only proceed if no explainResult

          // now check the status of what came back
          if (data && data.status == "success" && data.results && data.results.length > 0) try {
            //console.log("Advise success: " + JSON.stringify(data.results[0]));
              queryResult.advice = data.results[0].advice.adviseinfo;
          }
          // need to handle any exceptions that might occur
          catch (exception) {
            console.log("Exception analyzing advise plan: " + exception);
          }

          // if status != success
          else if (data.errors) {
            console.log("Advise errors: " + JSON.stringify(data.errors, null, '    '));
            queryResult.advice = "Error getting index advice, status: " + status;
            if (status == 504)
              queryResult.advice += " (server timeout)";
            if (data && _.isArray(data.errors))
              data.errors.forEach(function (err) {queryResult.advice += ", " + err.msg;});
          }
          else {
            console.log("Unknown advise response: " + JSON.stringify(resp));
            queryResult.advice = "Unknown response from server.";
          }
        },
        /* error response from $http, log error but otherwise ignore */
        function error(resp) {
          var data = resp.data, status = resp.status;
          //console.log("Advise error Data: " + JSON.stringify(data));
          //console.log("Advise error Status: " + JSON.stringify(status));
          queryResult.advice = "Error getting index advice, status: " + status;
          if (status == 504)
            queryResult.advice += " (server timeout)";
          if (data && _.isArray(data.errors))
            data.errors.forEach(function (err) {queryResult.advice += ", " + err.msg;});

          return(Promise.resolve()); // don't want to short circuit resolution of other promises
        });

        return advise_promise;
      }

      return(Promise.resolve("Query is not advisable"));
    }

    // convenience function to determine whether a query result has actionable advice
    function hasRecommendedIndex(queryResult) {
      if (!queryResult || !queryResult.advice || !_.isArray(queryResult.advice))
        return false;

      return(queryResult.advice.some(function (element) {
        return element.recommended_indexes &&
          (element.recommended_indexes.covering_indexes || element.recommended_indexes.indexes);
      }));
    }

    // split a set of semi-colon-delimited queries into an array of queries
    function multipleQueries(queryText) {
      var findSemicolons = /("(?:[^"\\]|\\.)*")|('(?:[^'\\]|\\.)*')|(\/\*(?:.|[\n\r])*\*\/)|(`(?:[^`]|``)*`)|((?:[^;"'`\/]|\/(?!\*))+)|(;)/g;
      var matchArray = findSemicolons.exec(queryText);
      var queryCount = 0;

      while (matchArray != null) {
        if (matchArray[0] == ';')
          if (queryCount++ > 1)
            return true;
        matchArray = findSemicolons.exec(queryText);
      }
      return false;
    }

    //
    // whenever a query finishes, we need to set the state to indicate the query
    // is not longer running.
    //

    function finishQuery(runningQuery) {
      // if we have an explain result and not a regular result, copy
      // the explain result to the regular result
      if (runningQuery.result == executingQueryTemplate.result) {
        runningQuery.result = runningQuery.explainResultText;
        runningQuery.data = runningQuery.explainResult;
      }

      runningQuery.currentQueryRequest = null;  // no query running
      runningQuery.busy = false; // enable the UI

      //console.log("Done with query: " + runningQuery.query + ", " + runningQuery.status);
    }

    //
    // manage metadata, including buckets, fields, and field descriptions
    //

    function updateQueryMonitoring(category) {

      var query1 = "select active_requests.*, meta().plan from system:active_requests";
      var query2 = "select completed_requests.*, meta().plan from system:completed_requests";
      var query3 = "select prepareds.* from system:prepareds";
      var query = "foo";

      switch (category) {
      case 1: query = query1; break;
      case 2: query = query2; break;
      case 3: query = query3; break;
      default: return;
      }

      var result = [];

      //console.log("Got query: " + query);

//      var config = {headers: {'Content-Type':'application/json','ns-server-proxy-timeout':20000}};
     // console.log("Running monitoring cat: " + category + ", query: " + payload.statement);

      return(executeQueryUtil(query,false))
      .then(function success(response) {
        var data = response.data;
        var status = response.status;
        var headers = response.headers;
        var config = response.config;

        if (data.status == "success") {
          result = data.results;

          // we need to reformat the duration values coming back
          // since they are in the most useless format ever.

          for (var i=0; i< result.length; i++) if (result[i].elapsedTime) {
            result[i].elapsedTime = qwQueryPlanService.convertTimeToNormalizedString(result[i].elapsedTime);
          }
        }
        else {
          result = [data.errors];
        }

        switch (category) {
        case 1:
          qwQueryService.monitoring.active_requests = result;
          qwQueryService.monitoring.active_updated = new Date();
          break;
        case 2:
          qwQueryService.monitoring.completed_requests = result;
          qwQueryService.monitoring.completed_updated = new Date();
          break;
        case 3:
          qwQueryService.monitoring.prepareds = result;
          qwQueryService.monitoring.prepareds_updated = new Date();
          break;
        }


      },
      function error(response) {
        var data = response.data;
        var status = response.status;
        var headers = response.headers;
        var config = response.config;

        //console.log("Mon Error Data: " + JSON.stringify(data));
        //console.log("Mon Error Status: " + JSON.stringify(status));
        //console.log("Mon Error Headers: " + JSON.stringify(headers));
        //console.log("Mon Error Config: " + JSON.stringify(config));
        var error = "Error with query monitoring";

        if (data && data.errors)
          error = error + ": " + JSON.stringify(data.errors);
        else if (status && _.isString(data))
          error = error + ", query service returned status: " + status + ", " + data;
        else if (status)
          error = error + ", query service returned status: " + status;

        logWorkbenchError(error);
//        console.log("Got error: " + error);

        switch (category) {
        case 1:
          qwQueryService.monitoring.active_requests = [{statment: error}];
          qwQueryService.monitoring.active_updated = new Date();
          break;
        case 2:
          qwQueryService.monitoring.completed_requests = [{statement: error}];
          qwQueryService.monitoring.completed_updated = new Date();
          break;
        case 3:
          qwQueryService.monitoring.prepareds = [{statement: error}];
          qwQueryService.monitoring.prepareds_updated = new Date();
          break;
        }

      });
    };

    //
    // whenever the system changes, we need to update the list of valid buckets
    //

    //$rootScope.$on("indexStatusURIChanged",updateBuckets/*function() {console.log("indexStatusURIChanged")}*/);
    $rootScope.$on("bucketUriChanged",updateBuckets);
    $rootScope.$on("checkBucketCounts",updateBucketCounts);

    function updateBuckets(event,data) {
      validateQueryService.getBucketsAndNodes(updateBucketsCallback);
    }

    // get the number of docs in each bucket for which we have access
    function updateBucketCounts() {
      // build a query to get the doc count for each bucket that we know about
      var queryString = "select raw {";
      var bucketCount = 0;
      qwQueryService.buckets.forEach(function (bucket) {
        if (!bucket.schema_error) {
          if (bucketCount > 0) // second and subsequent buckets need a comma
            queryString += ',';

          bucketCount++;
          queryString += '"' + bucket.id + '" : (select raw count(*) from `' + bucket.id + '`)[0]';
        }
      });
      queryString +=  '}';

      // run the query, extract the document counts
      executeQueryUtil(queryString, false)
      .then(function success(resp) {
        if (resp && resp.data && resp.data.results && resp.data.results.length)
          qwQueryService.buckets.forEach(function (bucket) {
            if (_.isNumber(resp.data.results[0][bucket.id]))
              bucket.count = resp.data.results[0][bucket.id];
          });
      },
      function error(resp) {
        console.log("bucket count error: " + JSON.stringify(resp));
      });
    }

    function updateBucketsCallback() {
      // make sure we have a query node
      if (!validateQueryService.valid())
        return;

      //console.log("Inside updateBucketsCallback");
      // use a query to get buckets with a primary index

      var queryText = qwConstantsService.keyspaceQuery;

      res1 = executeQueryUtil(queryText, false)
      .then(function success(resp) {
        var data = resp.data, status = resp.status;

        // remember the counts of each bucket so the screen doesn't blink when recomputing counts
        var bucket_counts = {};
        for (var i=0; i < qwQueryService.buckets.length; i++) {

          bucket_counts[qwQueryService.buckets[i].id] = qwQueryService.buckets[i].count;
        }

        // initialize the data structure for holding all the buckets
        qwQueryService.buckets.length = 0;
        qwQueryService.bucket_errors = null;
        qwQueryService.bucket_names.length = 0;
        qwQueryService.autoCompleteTokens = {};

        if (data && data.results) for (var i=0; i< data.results.length; i++) {
          var bucket = data.results[i];
          bucket.expanded = false;
          bucket.schema = [];
          bucket.indexes = [];
          bucket.validated = !validateQueryService.validBuckets ||
            _.indexOf(validateQueryService.validBuckets(),bucket.id) != -1 ||
            _.indexOf(validateQueryService.validBuckets(),".") != -1;
          bucket.count = bucket_counts[bucket.id];
          //console.log("Got bucket: " + bucket.id + ", valid: " + bucket.validated);
          if (bucket.validated) {
            qwQueryService.buckets.push(bucket); // only include buckets we have access to
            qwQueryService.bucket_names.push(bucket.id);
          }
          addToken(bucket.id,"bucket");
          addToken('`' + bucket.id + '`',"bucket");
        }

        refreshAutoCompleteArray();

        //
        // Should we go get information for each bucket?
        //

        if (qwConstantsService.showSchemas && qwQueryService.options.auto_infer)
          getInfoForBucketBackground(qwQueryService.buckets,0);

        /////////////////////////////////////////////////////////////////////////
        // now run a query to get the list of indexes
        /////////////////////////////////////////////////////////////////////////

        if (qwConstantsService.showSchemas) {
          queryText = 'select indexes.* from system:indexes where state = "online"';

          res1 = executeQueryUtil(queryText, false)
          //res1 = $http.post("/_p/query/query/service",{statement : queryText})
          .then(function (resp) {
            var data = resp.data, status = resp.status;

            //console.log("Got index info: " + JSON.stringify(data));

            if (data && _.isArray(data.results)) {
              qwQueryService.indexes = data.results;
              // make sure each bucket knows about each relevant index
              for (var i=0; i < data.results.length; i++) {
                addToken(data.results[i].name,'index');
                for (var b=0; b < qwQueryService.buckets.length; b++)
                  if (data.results[i].keyspace_id === qwQueryService.buckets[b].id) {
                    qwQueryService.buckets[b].indexes.push(data.results[i]);
                    break;
                  }
              }
            }

            refreshAutoCompleteArray();
          },

          // error status from query about indexes
          function index_error(resp) {
            var data = resp.data, status = resp.status;

            //console.log("Ind Error Data: " + JSON.stringify(data));
            //console.log("Ind Error Status: " + JSON.stringify(status));
            //console.log("Ind Error Headers: " + JSON.stringify(headers));
            //console.log("Ind Error statusText: " + JSON.stringify(statusText));

            var error = "Error retrieving list of indexes";

            if (data && data.errors)
              error = error + ": " + data.errors;
            if (status)
              error = error + ", contacting query service returned status: " + status;
//          if (response && response.statusText)
//          error = error + ", " + response.statusText;

//            console.log(error);
            logWorkbenchError(error);

            qwQueryService.index_error = error;
          }
          );
        }

      },
      /* error response from $http */
      function error(resp) {
        var data = resp.data, status = resp.status;
//        console.log("Schema Error Data: " + JSON.stringify(data));
//        console.log("Schema Error Status: " + JSON.stringify(status));
//        console.log("Schema Error Headers: " + JSON.stringify(headers));
//        console.log("Schema Error Config: " + JSON.stringify(config));
        var error = "Error retrieving list of buckets";

        if (data && data.errors)
          error = error + ": " + JSON.stringify(data.errors);
        else if (status)
          error = error + ", contacting query service returned status: " + status;

        qwQueryService.buckets.length = 0;
        qwQueryService.autoCompleteTokens = {};
        qwQueryService.bucket_errors = error;
        logWorkbenchError(error);
      });


    }

    //
    // this method uses promises and recursion to get the schemas for a list of
    // buckets in sequential order, waiting for each one before moving on to the next.
    //

    function getInfoForBucketBackground(bucketList,currentIndex,countsOnly) {
      // if we've run out of buckets, nothing more to do except get the bucket counts
      if (currentIndex < 0 || currentIndex >= bucketList.length) {
        updateBucketCounts();
        return;
      }

      getSchemaForBucket(bucketList[currentIndex]) // get the schema, pause, then get the next one
      .then(function successCallback(response) {
        $timeout(function() {getInfoForBucketBackground(bucketList,currentIndex+1);},500);
      }, function errorCallback(response) {
        $timeout(function() {getInfoForBucketBackground(bucketList,currentIndex+1);},500);
      });
    }


    //
    // Get a schema for a given, named bucket.
    //

    function getSchemaForBucket(bucket) {

      //console.log("Getting schema for : " + bucket.id);

      //return $http(inferQueryRequest)
      return executeQueryUtil('infer \`' + bucket.id + '\`  with {"infer_timeout":5, "max_schema_MB":1};', false)
      .then(function successCallback(response) {
        //console.log("Done with schema for: " + bucket.id);
        //console.log("Schema status: " + response.status);
        //console.log("Schema data: " + JSON.stringify(response.data));

        if (_.isArray(response.data.warnings) && response.data.warnings.length > 0)
          bucket.schema_error = response.data.warnings[0].msg;

        bucket.schema.length = 0;

        if (!response || !response.data)
          bucket.schema_error = "Empty or invalid server response: ";
        else if (response.data.errors) {
          bucket.schema_error = "Infer error: ";
          if (_.isString(response.data.errors))
            bucket.schema_error += response.data.errors;
          else if (_.isArray(response.data.errors)) {
            response.data.errors.forEach(function(val) {
              if (val.msg) bucket.schema_error += val.msg + ' ';
              else bucket.schema_error += JSON.stringify(val) + ' ';
            });
          }
          else
            bucket.schema_error += JSON.stringify(response.data.errors);
        }
        else if (response.data.status == "stopped") {
          bucket.schema_error = "Infer error, query stopped on server.";
        }
        else if (response.data.status != "success") {
          bucket.schema_error = "Infer error: " + response.data.status;
        }
        else if (_.isString(response.data.results))
          bucket.schema_error = response.data.results;
        else {
          //console.log("Got schema: " + JSON.stringify(response.data.results));
          bucket.schema = response.data.results[0];

          var totalDocCount = 0;
          for (var i=0; i<bucket.schema.length; i++)
            totalDocCount += bucket.schema[i]['#docs'];

          getFieldNamesFromSchema(bucket.schema,"");
          getFieldNamesFromSchema(bucket.schema,bucket.name);
          truncateSchema(bucket.schema);
          refreshAutoCompleteArray();

          //console.log("for bucket: " + bucket.id + " got " + bucket.schema.length + " flavars, doc count: " + totalDocCount);
          bucket.totalDocCount = totalDocCount;

          for (var i=0; i<bucket.schema.length; i++)
            bucket.schema[i]['%docs'] = (bucket.schema[i]['#docs']/totalDocCount*100);

          // we have an array of columns that are indexed. Let's mark the individual
          // fields, now that we have a schema.
          bucket.indexed_fields = {};

          // each element of the sec_ind array is an array of field names, turn into a map
          _.forEach(bucket.sec_ind,function(elem) {
            _.forEach(elem,function(field) {
              // for now we can't handle objects inside arrays, so we'll just flag the
              // array field as having an index. Also, we need to remove any parens.
              var bracket = field.indexOf('[');
              if (bracket >= 0)
                field = field.substring(0,bracket);

              field = field.replace(/\(/g,'').replace(/\)/g,'');

              //console.log("Index on: " + field);
              bucket.indexed_fields[field] = true;
            })});

          for (var flavor=0; flavor<bucket.schema.length; flavor++) { // iterate over flavors
            markIndexedFields(bucket.indexed_fields, bucket.schema[flavor], "");
            bucket.schema[flavor].hasFields =
              (bucket.schema[flavor].properties && Object.keys(bucket.schema[flavor].properties).length > 0) ||
              bucket.schema[flavor].type;
          }

          //if (bucket.schema.length)
          //  bucket.schema.unshift({Summary: "Summary: " + bucket.schema.length + " flavors found, sample size "+ totalDocCount + " documents",
          //    hasFields: true});
        }

      }, function errorCallback(response) {
        var error = "Error getting schema for bucket: " + bucket.id;
        if (response)
          if (response.data && response.data.errors) {
            error += ", " + JSON.stringify(response.data.errors,null,'  ');
          }
          else if (response.status) {
            error += ", " + response.status;
            if (response.statusText)
              error += " " + response.statusText;
          }
          else
            error += JSON.stringify(response);

        bucket.schema_error = error;
      });

    };

    //
    // When we get the schema, we need to mark the indexed fields. We start at the top
    // level, but recursively traverse any subtypes, keeping track of the path that we
    // followed to get to the subtype.
    //

    function markIndexedFields(fieldMap, schema, path) {
      //console.log("marking schema size: "+schema.fields.length + " with path: " + path);

      _.forEach(schema['properties'], function(theField, field_name) {
        // in the list of indexed fields, the field names are quoted with back quotes
        var quoted_field_name = '`' + field_name + '`';
        if (path.length > 0)
          quoted_field_name = path + quoted_field_name;

        //console.log(" checking field: " + quoted_field_name);

        // are we in the index map?
        if (fieldMap[quoted_field_name]) {
          theField.indexed = true;
        }

        // do we have a subtype to traverse?
        if (theField.properties)
          markIndexedFields(fieldMap,theField,path + '`' + field_name + '`.');
      });
    };


    //
    // show an error dialog
    //

    function showErrorDialog(message,details_array) {
      var dialogScope = $rootScope.$new(true);
      dialogScope.error_title = "Error";
      dialogScope.error_detail = message;
      dialogScope.error_detail_array = details_array;

      $uibModal.open({
        templateUrl: '../_p/ui/query/ui-current/password_dialog/qw_query_error_dialog.html',
        scope: dialogScope
      });
    }

    function showWarningDialog(message,details_array) {
      var dialogScope = $rootScope.$new(true);
      dialogScope.error_title = "Warning";
      dialogScope.error_detail = message;
      dialogScope.error_detail_array = details_array;

      $uibModal.open({
        templateUrl: '../_p/ui/query/ui-current/password_dialog/qw_query_error_dialog.html',
        scope: dialogScope
      });
    }

    //
    // load state from storage if possible
    //

    loadStateFromStorage();

    //
    // when we are initialized, get the list of buckets
    //

    $timeout(function(){
      updateBuckets();
    },500);

    //
    // all done creating the service, now return it
    //

    return qwQueryService;
  }

})();
