(function () {
  "use strict";

  angular
    .module('mnUserRolesList', [
      "mnFilters",
      "mnUserRolesService",
      "mnPromiseHelper"
    ])
    .directive('mnUserRolesList', mnUserRolesListDirective);

   function mnUserRolesListDirective() {
    var mnUserRolesList = {
      restrict: 'E',
      scope: {
        rolesToEnable: "=?",
        selectedRoles: "=",
        selectedWrappers: "=?",
        selectedGroupsRoles: "=?"
      },
      templateUrl: 'app/components/directives/mn_user_roles_list.html',
      controller: mnUserRolesListController,
      controllerAs: "mnThisCtl",
      bindToController: true
    };

     return mnUserRolesList;

     function mnUserRolesListController(mnUserRolesService, mnPromiseHelper) {
       var vm = this;

       vm.openedWrappers = {};

       vm.getUIID = mnUserRolesService.getRoleUIID;

       vm.toggleWrappers = toggleWrappers;
       vm.isRoleDisabled = isRoleDisabled;
       vm.onCheckChange = onCheckChange;
       vm.getGroupsList = getGroupsList;
       vm.hasGroups = hasGroups;

       activate();

       function hasGroups(id) {
         if (vm.selectedGroupsRoles && vm.selectedGroupsRoles[id]) {
           return !!Object.keys(vm.selectedGroupsRoles[id]).length;
         } else {
           return false;
         }
       }

       function getGroupsList(id) {
         return Object.keys(vm.selectedGroupsRoles[id]).join(", ");
       }

       function activate() {
         vm.openedWrappers[vm.getUIID({role: "admin"}, true)] = true;

         mnPromiseHelper(vm, mnUserRolesService.getRoles())
           .showSpinner()
           .onSuccess(function (roles) {
             vm.allRoles = roles;
             vm.rolesTree = mnUserRolesService.getRolesTree(roles);
             if (vm.rolesToEnable) {
               // user.roles
               vm.rolesToEnable.forEach(function (role) {
                 var id = vm.getUIID(role);
                 vm.selectedRoles[id] = true;
                 onCheckChange(role, id);
               });
             }
           });
       }

       function onCheckChange(role, id) {
         var selectedRoles;
         if (vm.selectedRoles[id]) {
           if (role.role === "admin") {
             selectedRoles = {};
             selectedRoles[id] = true;
             vm.selectedRoles = selectedRoles;
           } else if (role.bucket_name === "*") {
             vm.allRoles.forEach(function (item) {
               if (item.bucket_name !== undefined &&
                   item.bucket_name !== "*" &&
                   item.role === role.role) {
                 vm.selectedRoles[vm.getUIID(item)] = false;
               }
             });
           }
         }

         reviewSelectedWrappers();
       }

       function reviewSelectedWrappers() {
         vm.selectedWrappers =
           mnUserRolesService.reviewSelectedWrappers(vm.selectedRoles, vm.selectedGroupsRoles);
       }

       function isRoleDisabled(role) {
         return (role.role !== 'admin' && vm.selectedRoles[vm.getUIID({role: 'admin'})]) ||
           (role.bucket_name !== '*' &&
            vm.selectedRoles[vm.getUIID({role: role.role, bucket_name: '*'})]);
       }

       function toggleWrappers(id, value) {
         vm.openedWrappers[id] = !vm.openedWrappers[id];
       }
     }
  }
})();
