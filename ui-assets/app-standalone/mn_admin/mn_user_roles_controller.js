(function () {
  "use strict";

  angular
    .module("mnUserRoles", [
      "mnUserRolesService",
      "mnUserRolesList",
      "mnHelper",
      "mnPromiseHelper",
      "mnPoll",
      "mnSortableTable",
      "mnSpinner",
      "ui.select",
      "mnEqual",
      "mnFilters",
      "mnAutocompleteOff",
      "mnFocus"
    ])
    .controller("mnUserRolesController", mnUserRolesController);

  function mnUserRolesController($scope, $uibModal, mnPromiseHelper, mnUserRolesService, mnPoller, mnHelper, $state, poolDefault) {
    var vm = this;

    vm.deleteUser = deleteUser;
    vm.editUser = editUser;
    vm.resetUserPassword = resetUserPassword;

    vm.filterField = "";

    vm.stateParams = $state.params;

    vm.pageSize = $state.params.pageSize;
    vm.pageSizeChanged = pageSizeChanged;
    vm.parseGroupNames = parseGroupNames;
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

    function parseGroupNames(group) {
      return _.uniq(group.groups.concat(group.external_groups)).join(", ");
    }

    function sortByChanged(sortBy) {
      $state.go('.', {
        order: $state.params.sortBy != sortBy ? "asc" :
          $state.params.order === "asc" ? "desc" : "asc",
        sortBy: sortBy
      });
    }

    function activate() {
      $scope.$watchGroup(["userRolesCtl.stateParams.order",
                          "userRolesCtl.stateParams.sortBy",
                          "userRolesCtl.stateParams.substr"], _.debounce(function () {
                            $scope.$broadcast("reloadRolesPoller");
                          }, 500, {leading: true}));

      $scope.$watch('userRolesCtl.filterField', _.debounce(function () {
        $state.go('.', {
          substr: vm.filterField || undefined
        })
      }, 500, {leading: true}), true);

      mnHelper.initializeDetailsHashObserver(vm, 'openedUsers', '.');

      mnPromiseHelper(vm, mnUserRolesService.getRoles())
        .applyToScope(function (roles) {
          mnPromiseHelper(vm, mnUserRolesService.getRolesByRole(roles))
            .applyToScope("rolesByRole");
        });

      if (poolDefault.saslauthdEnabled) {
        mnPromiseHelper(vm, mnUserRolesService.getSaslauthdAuth())
          .applyToScope("saslauthdAuth");
      }


      if (poolDefault.compat.atLeast65 && poolDefault.isEnterprise) {
        new mnPoller($scope, function () {
          return mnUserRolesService.getLdapSettings();
        })
          .subscribe("ldapSettings", vm)
          .setInterval(10000)
          .reloadOnScopeEvent("reloadLdapSettings")
          .cycle();
      }

      new mnPoller($scope, function () {
        return mnUserRolesService.getState($state.params);
      })
        .subscribe("state", vm)
        .setInterval(10000)
        .reloadOnScopeEvent("reloadRolesPoller")
        .cycle();
    }

    function editUser(user) {
      $uibModal.open({
        templateUrl: 'app/mn_admin/mn_user_roles_add_dialog.html',
        controller: 'mnUserRolesAddDialogController as userRolesAddDialogCtl',
        resolve: {
          user: mnHelper.wrapInFunction(user),
          isLdapEnabled: function () {
            return (vm.saslauthdAuth && vm.saslauthdAuth.enabled) || (vm.ldapSettings && vm.ldapSettings.data.authenticationEnabled);
          }
        }
      });
    }
    function resetUserPassword(user) {
      $uibModal.open({
        templateUrl: 'app/mn_admin/mn_user_roles_reset_password_dialog.html',
        controller: 'mnUserRolesResetPasswordDialogController as userRolesResetPasswordDialogCtl',
        resolve: {
          user: mnHelper.wrapInFunction(user)
        }
      });
    }
    function deleteUser(user) {
      $uibModal.open({
        templateUrl: 'app/mn_admin/mn_user_roles_delete_dialog.html',
        controller: 'mnUserRolesDeleteDialogController as userRolesDeleteDialogCtl',
        resolve: {
          user: mnHelper.wrapInFunction(user)
        }
      });
    }
  }
})();
