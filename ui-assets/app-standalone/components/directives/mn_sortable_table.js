(function () {
  "use strict";

  angular
    .module('mnSortableTable', [])
    .directive('mnSortableTable', mnSortableTableDirective)
    .directive('mnSortableTitle', mnSortableTitleDirective)

  function mnSortableTitleDirective($compile) {
    var mnSortableTitle = {
      require: '^mnSortableTable',
      transclude: 'element',
      restrict: 'A',
      link: link,
      scope: {
        sortFunction: "&?",
        mnSortableTitle: "@?"
      }
    };

    return mnSortableTitle;

    function link($scope, $element, $attrs, ctl, $transclude) {
      $scope.mnSortableTable = ctl;
      if ($attrs.sortByDefault) {
        ctl.setOrderOrToggleInvert(
          $scope.sortFunction || $scope.mnSortableTitle,
          $scope.mnSortableTitle
        );
      }

      $transclude(function (cloned) {
        cloned.attr(
          'ng-click',
          'mnSortableTable.setOrderOrToggleInvert(sortFunction || mnSortableTitle, mnSortableTitle)'
        );
        cloned.attr(
          'ng-class',
          '{"dynamic-active": mnSortableTable.isOrderBy("'+ $scope.mnSortableTitle +'"),'
         + '"dynamic-inverted": mnSortableTable.isOrderBy("'+ $scope.mnSortableTitle +'") && mnSortableTable.sortableTableProperties.invert}'
        );
        cloned.removeAttr('mn-sortable-title');

        $element.after($compile(cloned)($scope));
      });
    }
  }

  function mnSortableTableDirective() {
     var mnSortableTable = {
       transclude: 'element',
       restrict: 'A',
       link: link,
       controller: controller,
       controllerAs: "mnSortableTable"
    };

    return mnSortableTable;

    function controller($scope, $element, $attrs, $parse) {
      var currentSortableTitle;
      var currentOrderByStringOrFunction;
      var vm = this;

      vm.sortableTableProperties = {
        orderBy: null,
        invert: null
      };
      vm.setOrderOrToggleInvert = setOrderOrToggleInvert;
      vm.isOrderBy = isOrderBy;
      vm.sortableTableProperties.orderBy = orderBy;

      function orderBy(value) {
        if (angular.isFunction(currentOrderByStringOrFunction)) {
          return currentOrderByStringOrFunction({value: value});
        } else {
          return $parse(currentOrderByStringOrFunction)(value);
        }
      }

      function isOrderBy(name) {
        return currentSortableTitle === name;
      }
      function setOrderOrToggleInvert(orderBy, name) {
        if (isOrderBy(name)) {
          vm.sortableTableProperties.invert = !vm.sortableTableProperties.invert;
        } else {
          vm.sortableTableProperties.invert = false;
        }
        setOrder(orderBy, name);
      }
      function setOrder(orderBy, name) {
        currentSortableTitle = name;
        currentOrderByStringOrFunction = orderBy;
      }
    }

    function link($scope, $element, $attrs, ctl, $transclude) {
      $transclude(function (cloned) {
        $element.after(cloned);
      });
    }
  }
})();
