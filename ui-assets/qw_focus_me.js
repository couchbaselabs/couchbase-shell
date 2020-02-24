(function() {

  //directive to allow password field to grab focus

  angular.module('app').directive('qwFocusMe', function ($timeout,$parse) {
    return {
      link: function (scope, element, attrs, model) {
        var model = $parse(attrs.focusMe);
        scope.$watch(model,function() {
          $timeout(function () {
            element[0].focus();
          });
        });
      }
    };
  });


})();