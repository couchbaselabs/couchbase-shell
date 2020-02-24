(function () {
  "use strict";

  angular
    .module("mnSelectableNodesList", [
      "mnFilters",
      "mnSearch"
    ])
    .directive("mnSelectableNodesList", mnSelectableNodesListDirective);

  function mnSelectableNodesListDirective() {
    var mnSelectableNodesList = {
      restrict: "A",
      scope: {
        nodes: "=",
        mnIsNodeDisabled: "&?",
        mnGroups: "=?",
        mnSelectedNodesHolder: "="
      },
      templateUrl: "app/components/directives/mn_selectable_nodes_list.html",
      controller: mnSelectableNodesListController,
      controllerAs: "mnThisCtl",
      bindToController: true
    };

    return mnSelectableNodesList;

    function mnSelectableNodesListController($scope) {
      var vm = this;

      vm.toggleAll = toggleAll;
      vm.findEnabled = findEnabled;
      vm.getGroupName = getGroupName;
      vm.areAllChecked = areAllChecked;

      function areAllChecked(bool) {
        return !!$scope.filteredNodes && !!$scope.filteredNodes.length && !findEnabled(bool);
      }

      function getGroupName(node) {
        return !!vm.mnGroups && vm.mnGroups[node.hostname].name;
      }

      function findEnabled(bool) {
        return !!_.find($scope.filteredNodes, function (node) {
          if (vm.mnIsNodeDisabled) {
            return !vm.mnIsNodeDisabled({node:node}) &&
              (!!vm.mnSelectedNodesHolder[node.otpNode] === bool);
          } else {
            return !!vm.mnSelectedNodesHolder[node.otpNode] === bool;
          }
        });
      }

      function setEnabled(bool) {
        $scope.filteredNodes.forEach(function (node) {
          if (vm.mnIsNodeDisabled) {
            if (!vm.mnIsNodeDisabled({node:node})) {
              vm.mnSelectedNodesHolder[node.otpNode] = bool;
            }
          } else {
            vm.mnSelectedNodesHolder[node.otpNode] = bool;
          }
        });
      }

      function toggleAll() {
        setEnabled(findEnabled(false));
      }
    }
  }
})();
