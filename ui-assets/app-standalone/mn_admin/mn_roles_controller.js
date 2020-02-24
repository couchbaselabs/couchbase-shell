(function () {
  "use strict";

  angular
    .module("mnUserRoles")
    .controller("mnRolesController", mnRolesController);

  function mnRolesController(poolDefault, mnHelper, $uibModal, $q) {
    var vm = this;
    vm.addUser = addUser;
    vm.addRolesGroup = addRolesGroup;
    vm.addLDAP = addLDAP;

    function addUser() {
      $uibModal.open({
        templateUrl: 'app/mn_admin/mn_user_roles_add_dialog.html',
        controller: 'mnUserRolesAddDialogController as userRolesAddDialogCtl',
        resolve: {
          user: mnHelper.wrapInFunction(undefined),
          isLdapEnabled: function (mnUserRolesService) {
            return $q.all([
              poolDefault.saslauthdEnabled ?
                mnUserRolesService.getSaslauthdAuth() : $q.when(),
              (poolDefault.isEnterprise && poolDefault.compat.atLeast65) ?
                mnUserRolesService.getLdapSettings() : $q.when()
            ]).then(function (resp) {
              return (resp[0] && resp[0].enabled) || (resp[1] && resp[1].data.authenticationEnabled);
            });
          }
        }
      });
    }

    function addLDAP() {
      $uibModal.open({
        templateUrl: 'app/mn_admin/mn_add_ldap_dialog.html',
        controller: 'mnAddLDAPDialogController as addLdapDialogCtl'
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

  }
})();
