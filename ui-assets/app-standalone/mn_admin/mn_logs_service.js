(function () {
  "use strict";

  angular.module('mnLogsService', [
    'mnLogsCollectInfoService',
    'ui.bootstrap'
  ]).service('mnLogsService', mnLogsServiceFactory);

  function mnLogsServiceFactory($http, $rootScope, $uibModal) {
    var mnLogsService = {
      getLogs: getLogs,
      showClusterInfoDialog: showClusterInfoDialog
    };

    return mnLogsService;

    function getLogs() {
      return $http.get('/logs');
    }

    function getClusterInfo() {
      return $http.get('/pools/default/terseClusterInfo');
    }

    function showClusterInfoDialog() {
      return getClusterInfo().then(function (resp) {
        var scope = $rootScope.$new();
        scope.info = JSON.stringify(resp.data, null, 2);
        return $uibModal.open({
          templateUrl: 'app/mn_admin/mn_cluster_info_dialog.html',
          scope: scope
        });
      });
    }
  }
})();
