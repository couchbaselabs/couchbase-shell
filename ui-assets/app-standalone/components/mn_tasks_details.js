(function () {
  "use strict";

  angular.module('mnTasksDetails', [
  ]).factory('mnTasksDetails', mnTasksDetailsFactory);

  function mnTasksDetailsFactory($http, $cacheFactory) {
    var mnTasksDetails = {
      get: get,
      clearCache: clearCache,
      getFresh: getFresh,
      getRebalanceReport: getRebalanceReport,
      clearRebalanceReportCache: clearRebalanceReportCache
    };

    return mnTasksDetails;

    function getRebalanceReport(url) {
      return $http({
        url: url || "/logs/rebalanceReport",
        cache: true,
        method: 'GET'
      }).then(null,function () {
        return {data: {stageInfo: {}}};
      });
    }

    function clearRebalanceReportCache(url) {
      $cacheFactory.get('$http').remove(url || "/logs/rebalanceReport");
      return this;
    }

    function get(mnHttpParams) {
      return $http({
        url: '/pools/default/tasks',
        method: 'GET',
        cache: true,
        mnHttp: mnHttpParams
      }).then(function (resp) {
        var rv = {};
        var tasks = resp.data;

        rv.tasks = tasks;
        rv.tasksXDCR = _.filter(tasks, detectXDCRTask);
        rv.tasksRecovery = _.detect(tasks, detectRecoveryTasks);
        rv.tasksRebalance = _.detect(tasks, detectRebalanceTasks);
        rv.tasksWarmingUp = _.filter(tasks, detectWarmupTask);
        rv.inRebalance = !!(rv.tasksRebalance && rv.tasksRebalance.status === "running");
        rv.inRecoveryMode = !!rv.tasksRecovery;
        rv.isLoadingSamples = !!_.detect(tasks, detectLoadingSamples);
        rv.stopRecoveryURI = rv.tasksRecovery && rv.tasksRecovery.stopURI;
        rv.isSubtypeGraceful = rv.tasksRebalance.subtype === 'gracefulFailover';
        rv.running = _.filter(tasks, function (task) {
          return task.status === "running";
        });
        rv.isOrphanBucketTask = !!_.detect(tasks, detectOrphanBucketTask);

        return rv;
      });
    };

    function detectXDCRTask(taskInfo) {
      return taskInfo.type === 'xdcr';
    }

    function detectOrphanBucketTask(taskInfo) {
      return taskInfo.type === "orphanBucket";
    }

    function detectRecoveryTasks(taskInfo) {
      return taskInfo.type === "recovery";
    }

    function detectRebalanceTasks(taskInfo) {
      return taskInfo.type === "rebalance";
    }

    function detectLoadingSamples(taskInfo) {
      return taskInfo.type === "loadingSampleBucket" && taskInfo.status === "running";
    }

    function detectWarmupTask(task) {
      return task.type === 'warming_up' && task.status === 'running';
    }

    function clearCache() {
      $cacheFactory.get('$http').remove('/pools/default/tasks');
      return this;
    }

    function getFresh(mnHttpParams) {
      return mnTasksDetails.clearCache().get(mnHttpParams);
    }
  }
})();
