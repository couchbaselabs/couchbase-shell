(function () {
  "use strict";

  angular
    .module('mnGroups', [
      'mnGroupsService',
      'mnSpinner',
      'mnHelper',
      'mnPoll',
      'mnPromiseHelper',
      'mnDragAndDrop',
      'ui.bootstrap',
      'mnFilters',
      'mnAlertsService',
      'mnPoolDefault',
      'mnElementCrane'
    ])
    .controller('mnGroupsController', mnGroupsController);

    function mnGroupsController($scope, $uibModal, mnGroupsService, mnPromiseHelper, mnHelper, mnPoller, mnMakeSafeForCSSFilter, mnNaturalSortingFilter, $window, mnAlertsService, mnPoolDefault) {
      var vm = this;

      vm.createGroup = createGroup;
      vm.deleteGroup = deleteGroup;
      vm.applyChanges = applyChanges;
      vm.reloadState = mnHelper.reloadState;
      vm.changeNodeGroup = changeNodeGroup;
      vm.disableApplyChangesBtn = true;

      activate();

      function applyChanges() {
        mnPromiseHelper($scope, mnGroupsService.applyChanges(vm.state.uri, vm.state.currentGroups))
          .reloadState("app.admin.servers.list.groups")
          .showGlobalSuccess("Group changes applied successfully!")
          .getPromise()
          .then(null, function (resp) {
            if (resp.status === 409) {
              vm.disableAddGroupBtn = true;
              vm.disableApplyChangesBtn = true;
              vm.revisionMismatch = true;
            } else {
              mnAlertsService.showAlertInPopup(resp.data, 'error');
            }
          });
      }

      function isGroupsEqual() {
        return _.isEqual(vm.state.initialGroups, vm.state.currentGroups);
      }

      function deleteGroup(group) {
        if (isGroupsEqual()) {
          return $uibModal.open({
            templateUrl: 'app/mn_admin/mn_groups_delete_dialog.html',
            controller: 'mnGroupsDeleteDialogController as groupsDeleteDialogCtl',
            resolve: {
              group: mnHelper.wrapInFunction(group)
            }
          });
        } else {
          $window.scrollTo(0, 0);
          vm.serverGroupsWarnig = true;
          vm.disableApplyChangesBtn = false;
        }
      }

      function createGroup(group) {
        return $uibModal.open({
          templateUrl: 'app/mn_admin/mn_groups_group_dialog.html',
          controller: 'mnGroupsGroupDialogController as groupsGroupDialogCtl',
          resolve: {
            group: mnHelper.wrapInFunction(group)
          }
        });
      }

      function changeNodeGroup(groupOld, groupNew, server) {
        if (groupOld === groupNew || groupNew === server.toGroupPending) {
          return;
        }
        var fromGroup = _.find(vm.state.currentGroups, function (cGroup) {
          return cGroup.name === groupOld;
        });

        var toGroup = _.find(vm.state.currentGroups, function (cGroup) {
          return cGroup.name === groupNew;
        });

        _.remove(fromGroup.nodes, function (node) {
          return node.hostname === server.hostname;
        });

        toGroup.nodes.push(server);

        if (server.toGroupPending === groupOld) {
          delete server.toGroupPending;
        } else {
          server.toGroupPending = toGroup.name
        }

        vm.disableApplyChangesBtn = false;
      }

      function activate() {
        mnPromiseHelper(vm, mnGroupsService.getGroupsState())
          .applyToScope("state");
      }
    }
})();
