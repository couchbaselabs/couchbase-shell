(function () {
  "use strict";

  /**
   * Service supporting access to UI applicable environment variables.
   */
  angular
    .module('mnEnv', [])
    .factory('mnEnv', mnEnvFactory);

  function mnEnvFactory($http) {

    var envUrl = '/_uiEnv';
    var envDefaults = {
      disable_autocomplete: true
    };
    return {
      loadEnv: loadEnv
    };

    /**
     * Invokes the server side REST API and returns a promise that fulfills
     * with a JSON object that fulfills with the complete set of environment
     * variables.
     * @returns Promise
     */
    function loadEnv() {
      return $http({method: 'GET', url: envUrl, cache: true}).then(
        function (resp) {
          return angular.extend({}, envDefaults, resp.data);
        });
    }
  }
})();
