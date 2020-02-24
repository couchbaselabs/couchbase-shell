(function () {
  "use strict";

  angular
    .module('mnDropdown', [])
    .directive('mnDropdown', mnDropdownDirective)
    .directive('mnDropdownItem', mnDropdownItemDirective)

  function mnDropdownItemDirective() {
    var mnDropdownItem ={
      require: '^^mnDropdown',
      restrict: 'E',
      scope: {
        mnItem: '='
      },
      link: link
    };

    return mnDropdownItem;

    function link(scope, element, attrs, mnDropdownCtl) {
      element.on("mousedown", onMousedown);
      element.on("click", onItemClick);
      element.on("mouseup", onMouseup);

      scope.$on("$destroy", function () {
        element.off("mousedown", onMousedown);
        element.off("mouseup", onMouseup);
        element.off("click", onItemClick);
      });

      function onItemClick() {
        mnDropdownCtl.onItemClick(scope.mnItem);
      }

      function onMousedown() {
        element.addClass("mousedowm");
      }

      function onMouseup() {
        element.removeClass("mousedowm");
      }

    }
  }
  function mnDropdownDirective($document) {
    var mnDropdown = {
      restrict: 'E',
      scope: {
        model: "=?",
        onClose: "&?",
        onSelect: "&?",
        iconClass: "@?"
      },
      transclude: {
        'select': '?innerSelect',
        'header': '?innerHeader',
        'body': 'innerBody',
        'footer': '?innerFooter'
      },
      templateUrl: "app/components/directives/mn_dropdown.html",
      controller: controller
    };

    return mnDropdown;

    function controller($scope, $transclude, $timeout) {
      $scope.toggleMenu = toggleMenu;
      $scope.isSlotFilled = $transclude.isSlotFilled;
      this.onItemClick = onItemClick;

      $scope.$on("$destroy", function () {
        $document.off('click', toggleMenu);
      });

      function closeMenu() {
        $scope.showMenu = false;
        $scope.onClose && $scope.onClose($scope.model && $scope.model.selected);
      }

      function openMenu() {
        $scope.showMenu = true;
      }

      function onItemClick(item) {
        $timeout(function () {
          $scope.model && ($scope.model.selected = item);
          $scope.onSelect && $scope.onSelect({scenario: item});
          toggleMenu();
        });
      }

      function documentClick() {
        $timeout(toggleMenu);
      }

      function toggleMenu($event) {
        $event && $event.stopPropagation && $event.stopPropagation();
        ($scope.showMenu ? closeMenu : openMenu)();
        outsideClick();
      }

      function outsideClick() {
        $document[$scope.showMenu ? "on" : "off"]('click', documentClick);
      }
    }
  }
})();
