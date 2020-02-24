(function () {
  angular
    .module('mnServersListItemDetailsService', [])
    .factory('mnServersListItemDetailsService', mnServersListItemDetailsFactory);

  function mnServersListItemDetailsFactory($http, $q) {
    var mnServersListItemDetailsService = {
      getNodeDetails: getNodeDetails,
      getNodeTasks: getNodeTasks,
      getBaseConfig: getBaseConfig
    };

    return mnServersListItemDetailsService;

    function getValue(value) {
      return parseFloat(Array.isArray(value) ?
                        value.slice().reverse().find(stat => stat != null) : value);
    }

    function getBaseConfig(title, used, total, used2) {
      used = getValue(used);
      total = getValue(total);
      used2 = getValue(used2);
      if (Number.isNaN(used) || Number.isNaN(total)) {
        return;
      }
      return {
        items: [{
          name: title,
          value: used
        }, {
          name: 'remaining',
          value: total - (Number.isNaN(used2) ? used : used2)
        }]
      };
    }

    function getNodeTasks(node, tasks) {
      if (!tasks || !node) {
        return;
      }
      var rebalanceTask = tasks.tasksRebalance.status === 'running' && tasks.tasksRebalance;
      return {
        warmUpTasks: _.filter(tasks.tasksWarmingUp, function (task) {
          return task.node === node.otpNode;
        }),
        detailedProgress: rebalanceTask.detailedProgress && rebalanceTask.detailedProgress.perNode && rebalanceTask.detailedProgress.perNode[node.otpNode]
      };
    }

    function getNodeDetails(node) {
      return $http({method: 'GET', url: '/nodes/' + encodeURIComponent(node.otpNode)}).then(function (resp) {
        return {
          details: resp.data
        };
      });
    }
  }
})();
