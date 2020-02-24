/**
 * This directive can be applied to an HTML input to ensure that 
 * the value within is valid JSON.
 */
/* global _, angular */
(function() {

  'use strict';
  angular.module('qwValidJson', []).
  directive('qwValidJson', function () {

    return {
      require: 'ngModel',
      link: function (scope, element, attr, mCtrl) {

        // make sure that JSON.parse doesn't throw a parse exception on the value
        function validateJson(value) {
          if (!value) {
            mCtrl.$setValidity('badJSON',true);
            return value;
          }

          else try {
            JSON.parse(value);
            mCtrl.$setValidity('badJSON',true);
            return value;
          } catch (syntaxError) {
            mCtrl.$setValidity('badJSON',false);			      
            return value;
          }
        }

        mCtrl.$parsers.push(validateJson);
      }
    };
  });

})();
