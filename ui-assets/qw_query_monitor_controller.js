(function() {


  angular.module('qwQuery').controller('qwQueryMonitorController', queryMonController);

  queryMonController.$inject = ['$http','$rootScope', '$scope', '$state', '$uibModal', '$timeout', 'qwQueryService',
                                'validateQueryService', 'mnAnalyticsService','qwQueryPlanService', 'mnPoller', 'mnStatisticsNewService',
                                'mnHelper', 'mnPermissions'];

  function queryMonController ($http, $rootScope, $scope, $state,$uibModal, $timeout, qwQueryService,
                               validateQueryService, mnAnalyticsService, qwQueryPlanService, mnPoller,mnStatisticsNewService,mnHelper,mnPermissions) {

    var qmc = this;

    //
    // Do we have a REST API to work with?
    //

    qmc.validated = validateQueryService;

    // should we show active, completed, or prepared queries?

    qmc.selectTab = qwQueryService.selectMonitoringTab;
    qmc.isSelected = qwQueryService.isMonitoringSelected;
    qmc.cancelQueryById = cancelQueryById;
    qmc.getCancelLabel = getCancelLabel;
    qmc.cancelledQueries = {}; // keep track of user-cancelled queries

    //
    // keep track of results from the server
    //

    qmc.monitoring = qwQueryService.monitoring;

    qmc.updatedTime = updatedTime;
    qmc.toggle_update = toggle_update;
    qmc.get_toggle_label = get_toggle_label;
    qmc.get_update_flag = function() {return(qwQueryService.getMonitoringAutoUpdate());}
    qmc.options = qwQueryService.getMonitoringOptions;

    qmc.getSummaryStat = getSummaryStat;

    qmc.vitals = {};
    qmc.vitals_names = ["request.per.sec.15min","request.per.sec.5min",
      "request.per.sec.1min","request_time.mean","request_time.median","memory_util",
      "cpu.user.percent","cores"];
    qmc.vitals_labels = ["requests/sec (15min)","requests/sec (5min)",
      "requests/sec (1min)","mean request time","median request time","memory util",
      "cpu utilization","# cores"];
    qmc.getVital = getVital;
    qmc.showPlan = showPlan;

    qmc.charts = [
      {
        stats: {"@query.query_requests": true},
        size: "tiny",
        specificStat: true
      },
      {
        stats: {"@query.query_avg_req_time": true},
        size: "tiny",
        specificStat: true
      },
      {
        stats: {"@query.query_avg_svc_time": true},
        size: "tiny",
        specificStat: true
      }
    ];

    qmc.statsConfig = {
        node: "all",
        zoom: 60000,
        step: 1,
        stats: ['query_requests_250ms','query_requests_500ms','query_requests_1000ms',
          'query_requests_5000ms']
    };

    qmc.openDetailedChartDialog = openDetailedChartDialog;

    function openDetailedChartDialog(c) {
      $state.params.scenarioBucket = qmc.buckets[1];
      $uibModal.open(
          {
            templateUrl: 'app/mn_admin/mn_statistics/mn_statistics_detailed_chart.html',
            controller: 'mnStatisticsDetailedChartController as detailedChartCtl',
            windowTopClass: "chart-overlay",
            resolve: {
              items: mnHelper.wrapInFunction({}),
              chart: mnHelper.wrapInFunction(qmc.charts[c])
            }
          });
    }

    //
    // sorting for each of the three result tables
    //

    qmc.update_active_sort = function(field) {
      if (qmc.options().active_sort_by == field)
        qmc.options().active_sort_reverse = !qmc.active_sort_reverse;
      else
        qmc.options().active_sort_by = field;

      qwQueryService.saveStateToStorage();
    };
    qmc.show_up_caret_active = function(field) {
      return(qmc.options().active_sort_by == field && qmc.options().active_sort_reverse);
    };
    qmc.show_down_caret_active = function(field) {
      return(qmc.options().active_sort_by == field && !qmc.options().active_sort_reverse);
    };

    qmc.update_completed_sort = function(field) {
      if (qmc.options().completed_sort_by == field)
        qmc.options().completed_sort_reverse = !qmc.options().completed_sort_reverse;
      else
        qmc.options().completed_sort_by = field;

      qwQueryService.saveStateToStorage();
    };
    qmc.show_up_caret_completed = function(field) {
      return(qmc.options().completed_sort_by == field && qmc.options().completed_sort_reverse);
    };
    qmc.show_down_caret_completed = function(field) {
      return(qmc.options().completed_sort_by == field && !qmc.options().completed_sort_reverse);
    };

    qmc.update_prepared_sort = function(field) {
      if (qmc.options().prepared_sort_by == field)
        qmc.options().prepared_sort_reverse = !qmc.options().prepared_sort_reverse;
      else
        qmc.options().prepared_sort_by = field;

      qwQueryService.saveStateToStorage();
    };
    qmc.show_up_caret_prepared = function(field) {
      return(qmc.options().completed_sort_by == field && qmc.options().prepared_sort_reverse);
    };
    qmc.show_down_caret_prepared = function(field) {
      return(qmc.options().prepared_sort_by == field && !qmc.options().prepared_sort_reverse);
    };

    //
    // show the plan info for a completed query
    //

    function showPlan(statement, plan) {
      var dialogScope = $rootScope.$new(true);
      dialogScope.planText = JSON.stringify(plan,null,'  ');
      dialogScope.show_plan = true;
      dialogScope.statement = statement;
      dialogScope.set_show_plan = function(val) {dialogScope.show_plan = val;};

      // analyze the plan
      try {
        var lists = qwQueryPlanService.analyzePlan(plan,null);
        dialogScope.plan =
        {explain: {plan: plan, text: statement},
            analysis: lists,
            plan_nodes: qwQueryPlanService.convertN1QLPlanToPlanNodes(plan, null, lists)
        };
      } catch (exception) {console.log("Got exception analyzing plan: " + JSON.stringify(exception))}

      dialogScope.acePlanOptions = {
          mode: 'json',
          showGutter: true,
          useWrapMode: true,
          onLoad: function (_editor) {
            _editor.$blockScrolling = Infinity;
            _editor.setReadOnly(true);
            _editor.renderer.setPrintMarginColumn(false); // hide page boundary lines
          },
          $blockScrolling: Infinity
      };

      // bring up the dialog
      var promise = $uibModal.open({
        templateUrl: '../_p/ui/query/ui-current/query_plan_viz/qw_plan_dialog.html',
        scope: dialogScope
      }).result;
    }

    //
    // cancel a running query
    //

    function cancelQueryById(requestId) {
      // remember that the query was cancelled
      qmc.cancelledQueries[requestId] = true;
      // do the cancel
      qwQueryService.cancelQueryById(requestId);
    }

    //
    // get a label for cancel, which changes when the user hits "cancel"
    //

    function getCancelLabel(requestId) {
      if (qmc.cancelledQueries[requestId])
        return("cancelling");
      else
        return("Cancel");
    }

    //
    // when was the data last updated?
    //

    function updatedTime() {
      var result;
      switch (qwQueryService.getMonitoringSelectedTab()) {
      case 1: result = qmc.monitoring.active_updated; break
      case 2: result = qmc.monitoring.completed_updated; break;
      case 3: result = qmc.monitoring.prepareds_updated; break;
      }

      if (_.isDate(result)) {
        var minutes = result.getMinutes() > 9 ? result.getMinutes() : "0" + result.getMinutes();
        var seconds = result.getSeconds() > 9 ? result.getSeconds() : "0" + result.getSeconds();
        var dateStr = result.toString();
        var zone = dateStr.substring(dateStr.length-4,dateStr.length-1);
        result = " " + result.getHours() + ":" + minutes + ":" + seconds + " " + zone;
      }

      return result;
    }

    //
    // call the activate method for initialization
    //

    activate();

    //
    //
    //

    function activate() {
      // get initial data for each panel
      if (qmc.monitoring.active_updated == "never")
        qwQueryService.updateQueryMonitoring(1);
      if (qmc.monitoring.completed_updated == "never")
        qwQueryService.updateQueryMonitoring(2);
      if (qmc.monitoring.prepareds_updated == "never")
        qwQueryService.updateQueryMonitoring(3);

      // runs the queries and gets query engine stats
      new mnPoller($scope, update).setInterval(5000).cycle(); // run update() every 5 seconds

      // subscribe to stats
      qmc.statsPoller = mnStatisticsNewService.subscribeUIStatsPoller(qmc.statsConfig,$scope);

      // Prevent the backspace key from navigating back. Thanks StackOverflow!
      $(document).unbind('keydown').bind('keydown', function (event) {
        var doPrevent = false;
        if (event.keyCode === 8) {
          var d = event.srcElement || event.target;
          if ((d.tagName.toUpperCase() === 'INPUT' &&
              (
                  d.type.toUpperCase() === 'TEXT' ||
                  d.type.toUpperCase() === 'PASSWORD' ||
                  d.type.toUpperCase() === 'FILE' ||
                  d.type.toUpperCase() === 'SEARCH' ||
                  d.type.toUpperCase() === 'EMAIL' ||
                  d.type.toUpperCase() === 'NUMBER' ||
                  d.type.toUpperCase() === 'DATE' )
          ) ||
          d.tagName.toUpperCase() === 'TEXTAREA') {
            doPrevent = d.readOnly || d.disabled;
          }
          else {
            doPrevent = true;
          }
        }

        if (doPrevent) {
          event.preventDefault();
        }
      });

    }

    function toggle_update() {
      qwQueryService.setMonitoringAutoUpdate(!qwQueryService.getMonitoringAutoUpdate());
    }

    function get_toggle_label() {
      if (qwQueryService.getMonitoringAutoUpdate())
        return("pause");
      else
        return("resume");
    }

    //
    // function to update the current data at regular intervals
    //

    function update() {
      // update the currently selected tab
      if (qwQueryService.getMonitoringAutoUpdate())
        qwQueryService.updateQueryMonitoring(qwQueryService.getMonitoringSelectedTab());

      // get the stats from the Query service
      $http({
        url: "../_p/query/admin/vitals",
        method: "GET"
      }).then(function success(resp) {
        if (resp && resp.status == 200 && resp.data) {
          //console.log("Got vitals: " + JSON.stringify(resp.data));
          qmc.vitals = resp.data;
          qmc.vitals.memory_util = Math.round((qmc.vitals["memory.usage"] / qmc.vitals["memory.system"]) * 100);
          qmc.vitals_updated_at = Date.now();
        }
      });

      // we need to pass in the name of a bucket to which we have access, even though
      // the query stats are not bucket-specific

      qmc.buckets = validateQueryService.validBuckets();
      //console.log("Got buckets: "+ JSON.stringify(qmc.buckets));
      if (_.isArray(qmc.buckets) && qmc.buckets.length > 1) {
        qmc.statsConfig.bucket = qmc.buckets[1];
      }

      return Promise.resolve();
    }

    //
    // for items like the number of queries > time over the past minute,
    // we would like a sum of all values from the array
    //

    function getSummaryStat(name) {
      var s = $scope.mnUIStats;
      if (s && s.stats && s.stats[name] && _.isArray(s.stats[name].aggregate)) {
          var sum = 0;
          s.stats[name].aggregate.forEach(function(n) {sum+=n});
          return(sum);
      }
      else
        return null;
    }

    //
    // the vitals might be numbers, but they might be strings indicating a duration
    // (e.g., "452.637ms"). Make sure all are returned as numbers
    //

    function getVital(name) {
      var val = qmc.vitals[name];
      //console.log("Got vital: " +name + " = "+ val);
      if (_.isString(val))
        return(qwQueryPlanService.convertTimeStringToFloat(val));
      else
        return(val);
    }

    //
    // all done, return the controller
    //

    return qmc;
  }


})();
