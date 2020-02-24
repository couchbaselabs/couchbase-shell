(function () {
  "use strict";

  angular
    .module("mnUserRoles")
    .controller("mnUserRolesAddDialogController", mnUserRolesAddDialogController);

  function mnUserRolesAddDialogController($scope, mnUserRolesService, $uibModalInstance, mnPromiseHelper, user, isLdapEnabled, mnPoolDefault, $timeout, mnHelper, $q) {
    var vm = this;
    vm.user = _.clone(user) || {domain: "local"};
    vm.userID = vm.user.id || 'New';
    vm.save = save;
    vm.isEditingMode = !!user;
    vm.isLdapEnabled = isLdapEnabled;
    vm.selectedRoles = {};
    vm.selectedGroupsRoles = {};
    vm.selectedGroups = {};

    vm.focusError = false;
    vm.getGroupTitle = getGroupTitle;
    vm.onGroupChanged = onGroupChanged;
    vm.selectedPanel = "roles";
    vm.lookupMembership =  _.debounce(lookupMembership, 500, {leading: true});
    vm.onDomainChanged = onDomainChanged;
    vm.isLookupEnabled = isLookupEnabled;

    activate();

    function onDomainChanged() {
      if (vm.user.domain === "external") {
        lookupMembership();
      } else {
        clearRoles();
      }
    }

    function lookupMembership() {
      vm.reloadUserRoles = true;
      mnUserRolesService.lookupLDAPUser(vm.user)
        .then(function (user) {
          clearRoles();
          applyUser(user.data);
          vm.isUserAvailable = true;
          vm.reloadUserRoles = false;
        }, function () {
          clearRoles();
          vm.isUserAvailable = false;
          vm.reloadUserRoles = false;
        });
    }

    function isLookupEnabled() {
      return !vm.isEditingMode && (vm.user.domain === 'external');
    }

    function clearRoles() {
      vm.selectedRoles = {};
      vm.selectedGroupsRoles = {};
      vm.selectedGroups = {};
      vm.externalGroups = {};
      delete vm.rolesToEnable;
    }

    function selectRoles(group, flag) {
      return function (role) {
        var id = mnUserRolesService.getRoleUIID(role);
        vm.selectedGroupsRoles[id] = vm.selectedGroupsRoles[id] || {};
        if (flag) {
          vm.selectedGroupsRoles[id][group] = flag;
        } else {
          delete vm.selectedGroupsRoles[id][group];
        }
        reviewSelectedWrappers();
      }
    }

    function reviewSelectedWrappers() {
      vm.selectedWrappers =
        mnUserRolesService.reviewSelectedWrappers(vm.selectedRoles, vm.selectedGroupsRoles);
    }

    function onGroupChanged(group) {
      if (vm.selectedGroups[group.id]) {
        group.roles.forEach(selectRoles(group.id, true));
      } else {
        group.roles.forEach(selectRoles(group.id, false));
      }

      if (vm.externalGroups[group.id]) {
        group.roles.forEach(selectRoles(group.id, true));
      }

      reviewSelectedWrappers();
    }

    function getGroupTitle(roles) {
      return roles && roles.map(function (v) {
        return vm.byRole[v.role + (v.bucket_name ? '[' + v.bucket_name + ']' : '')].name;
      }).join(',');
    }

    function getUserRoles(user) {
      return user.roles.filter(function (role) {
        return role.origins.find(function (origin) {
          return origin.type == "user";
        });
      });
    }

    function groupsToObject(groups) {
      return groups.reduce(function (acc, group) {
        acc[group] = true;
        return acc;
      }, {});
    }

    function activate() {
      vm.reloadUserRoles = true;
      $q.all([
        mnUserRolesService.getRolesByRole(),
        (mnPoolDefault.export.isEnterprise && mnPoolDefault.export.compat.atLeast65) ?
          mnUserRolesService.getRolesGroups() : $q.when()
      ]).then(function (resp) {
        vm.byRole = resp[0];
        vm.groups = resp[1] && resp[1].data;
        applyUser(vm.user);
        vm.reloadUserRoles = false;
      }, function () {
        vm.reloadUserRoles = false;
      });
    }

    function applyUser(user) {
      vm.selectedGroups = groupsToObject(user.groups || []);
      vm.externalGroups = groupsToObject(user.external_groups || []);

      if (user.roles) {
        vm.rolesToEnable = getUserRoles(user);
        user.roles.forEach(function (role) {
          var id = mnUserRolesService.getRoleUIID(role);
          vm.selectedGroupsRoles[id] = vm.selectedGroupsRoles[id] || {};
          role.origins.forEach(function (group) {
            if (group.type == "group") {
              vm.selectedGroupsRoles[id][group.name] = true;
            }
          });
        });
      }
      reviewSelectedWrappers();
    }

    function save() {
      if (vm.form.$invalid) {
        vm.focusError = true;
        return;
      }

      //example of the inсoming role
      //All Buckets (*)|Query and Index Services|query_insert[*]
      var roles = [];
      _.forEach(vm.selectedRoles, function (value, key) {
        if (value) {
          var path = key.split("|");
          roles.push(path[path.length - 1]);
        }
      });

      mnPromiseHelper(vm, mnUserRolesService.addUser(
        vm.user, roles, mnHelper.checkboxesToList(vm.selectedGroups), vm.isEditingMode,
      ), $uibModalInstance)
        .showGlobalSpinner()
        .catchErrors(function (errors) {
          vm.focusError = !!errors;
          vm.errors = errors;
        })
        .broadcast("reloadRolesPoller")
        .closeOnSuccess()
        .showGlobalSuccess("User saved successfully!");
    }
  }
})();
