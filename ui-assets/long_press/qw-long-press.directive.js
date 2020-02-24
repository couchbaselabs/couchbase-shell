/**
  * This directive permits a function to be called when the user does a
  * long press. Let's say your controller defines a function somewhere, e.g.:
  *     qc.pressFn = function() {console.log("long press happened");}
  * In your HTML you could have something like:
  *     <div qw-long-press="qc.pressFn">Press me a long time</div>
  * Then, if the user clicks on the label for more than 600ms, the function
  * is colled.
 */
/* global _, angular */
(function() {

	'use strict';
	angular.module('qwLongPress', []).directive('qwLongPress', function ($timeout) {
		return {
			restrict: 'A',
			scope: { qwLongPress: '&' },
			link: function (scope, element, $attrs) {
              element.bind('mousedown',function(event) {
                // ignore right-click
                if (event.button != 0)
                  return;

                scope.longPress = true;

                // let's see if we're still pressing after a certain amount of time
                scope.longPressPromise = $timeout(function() {
                  if (scope.longPress) {
                    scope.longPress = false;
                    scope.qwLongPress()();
                  }
                },600);
              });

              element.bind('mouseup',function(event) {
                scope.longPress = false;
                if (scope.longPressPromise)
                  $timeout.cancel(scope.longPressPromise);
              });

			}
		};
	});


})();

