(function () {
  "use strict";

  angular
    .module("mnRolesGroups", [
      "mnUserRolesService",
      "mnUserRolesList",
      "mnHelper",
      "mnPromiseHelper",
      "mnPoll",
      "mnSpinner",
      "ui.select",
      "mnEqual",
      "mnFilters",
      "mnAutocompleteOff",
      "mnFocus"
    ])
    .controller("mnRolesGroupsController", mnRolesGroupsController);

  function mnRolesGroupsController($scope, $uibModal, mnPromiseHelper, mnUserRolesService, mnPoller, mnHelper, $state, poolDefault, $timeout) {
    var vm = this;

    vm.addRolesGroup = addRolesGroup;
    vm.deleteRolesGroup = deleteRolesGroup;
    vm.editRolesGroup = editRolesGroup;

    vm.filterField = $state.params.substr;

    vm.stateParams = $state.params;

    vm.pageSize = $state.params.pageSize;
    vm.pageSizeChanged = pageSizeChanged;
    vm.sortByChanged = sortByChanged;
    vm.isOrderBy = isOrderBy;
    vm.isDesc = isDesc;

    activate();

    function isOrderBy(sortBy) {
      return sortBy === $state.params.sortBy;
    }

    function isDesc() {
      return $state.params.order === "desc";
    }

    function pageSizeChanged() {
      $state.go('.', {
        pageSize: vm.pageSize
      });
    }

    function sortByChanged(sortBy) {
      $state.go('.', {
        order: $state.params.sortBy != sortBy ? "asc" :
          $state.params.order === "asc" ? "desc" : "asc",
        sortBy: sortBy
      })
    }

    function activate() {
      $scope.$watch('rolesGroupsCtl.filterField', _.debounce(function () {
        $state.go('.', {
          substr: vm.filterField || undefined
        })
      }, 500, {leading: true}), true);

      $scope.$watchGroup(["rolesGroupsCtl.stateParams.order",
                          "rolesGroupsCtl.stateParams.sortBy",
                          "rolesGroupsCtl.stateParams.substr"], _.debounce(function () {
                            $scope.$broadcast("reloadRolesGroupsPoller");
                          }, 500, {leading: true}));

      mnHelper.initializeDetailsHashObserver(vm, 'openedRolesGroups', '.');

      mnPromiseHelper(vm, mnUserRolesService.getRoles())
        .applyToScope(function (roles) {
          mnPromiseHelper(vm, mnUserRolesService.getRolesByRole(roles))
            .applyToScope("rolesByRole");
        });

      var poller = new mnPoller($scope, function () {
        return mnUserRolesService.getRolesGroupsState($state.params);
      })
          .subscribe("state", vm)
          .setInterval(10000)
          .reloadOnScopeEvent("reloadRolesGroupsPoller")
          .cycle();
    }

    function editRolesGroup(rolesGroup) {
      $uibModal.open({
        templateUrl: 'app/mn_admin/mn_roles_groups_add_dialog.html',
        controller: 'mnRolesGroupsAddDialogController as rolesGroupsAddDialogCtl',
        resolve: {
          rolesGroup: mnHelper.wrapInFunction(rolesGroup)
        }
      });
    }
    function addRolesGroup() {
      $uibModal.open({
        templateUrl: 'app/mn_admin/mn_roles_groups_add_dialog.html',
        controller: 'mnRolesGroupsAddDialogController as rolesGroupsAddDialogCtl',
        resolve: {
          rolesGroup: mnHelper.wrapInFunction(undefined)
        }
      });
    }
    function deleteRolesGroup(rolesGroup) {
      $uibModal.open({
        templateUrl: 'app/mn_admin/mn_roles_groups_delete_dialog.html',
        controller: 'mnRolesGroupsDeleteDialogController as rolesGroupsDeleteDialogCtl',
        resolve: {
          rolesGroup: mnHelper.wrapInFunction(rolesGroup)
        }
      });
    }
  }
})();
