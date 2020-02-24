(function () {
  "use strict";

  angular
    .module('mnAlertsService', ['ui.bootstrap', 'mnFilters'])
    .service('mnAlertsService', mnAlertsServiceFactory);

  function mnAlertsServiceFactory($uibModal, $rootScope, $timeout) {
    var alerts = [];
    var alertsHistory = [];
    var clientAlerts = {
      hideCompatibility: false
    };
    var mnAlertsService = {
      setAlert: setAlert,
      formatAndSetAlerts: formatAndSetAlerts,
      showAlertInPopup: showAlertInPopup,
      alerts: alerts,
      removeItem: removeItem,
      isNewAlert: isNewAlert,
      clientAlerts: clientAlerts
    };

    return mnAlertsService;

    function showAlertInPopup(message, title) {
      var scope = $rootScope.$new();
      scope.alertsCtl = {
        message: message
      };
      scope.title = title;
      return $uibModal.open({
        scope: scope,
        templateUrl: "app/components/mn_alerts_popup_message.html"
      }).result;
    }

    function isNewAlert(item) {
      var findedItem = _.find(alertsHistory, item);
      return _.indexOf(alertsHistory, findedItem) === -1;
    }

    function startTimer(item, timeout) {
      return $timeout(function () {
        removeItem(item);
      }, parseInt(timeout, 10));
    }

    function removeItem(item) {
      var index = _.indexOf(alerts, item);
      item.timeout && $timeout.cancel(item.timeout);
      alerts.splice(index, 1);
    }

    function setAlert(type, message, timeout, id) {
      var item = {
        type: type || 'error',
        msg: message,
        id: id
      };

      //in case we get alert with the same message
      //but different id find and remove it
      var findedItem = _.find(alerts, {
        type: type,
        msg: message
      });

      if (findedItem) {
        removeItem(findedItem);
      }

      alerts.push(item);
      alertsHistory.push(item);

      if (timeout) {
        item.timeout = startTimer(item, timeout);
      }
    }
    function formatAndSetAlerts(incomingAlerts, type, timeout) {
      timeout = timeout || (60000 * 5);
      if ((angular.isArray(incomingAlerts) && angular.isString(incomingAlerts[0])) ||
          angular.isObject(incomingAlerts)) {
        angular.forEach(incomingAlerts, function (msg) {
          setAlert(type, msg, timeout);
        });
      }

      if (angular.isString(incomingAlerts)) {
        setAlert(type, incomingAlerts, timeout);
      }
    }
  }
})();
