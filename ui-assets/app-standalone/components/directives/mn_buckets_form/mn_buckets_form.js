(function () {
  "use strict";

  angular
    .module('mnBucketsForm', [
      'mnFocus',
      'mnBucketsDetailsService',
      'mnFilters',
      'mnAutocompleteOff',
      'mnPromiseHelper',
      'mnBarUsage',
      'mnUserRolesService',
      'mnPermissions'
    ])
    .directive('mnBucketsForm', mnBucketsFormDirective);

  function mnBucketsFormDirective($http, mnBucketsDetailsDialogService, mnPromiseHelper, mnUserRolesService, $q, mnPermissions, $state) {

    var mnBucketsForm = {
      restrict: 'A',
      scope: {
        bucketConf: '=',
        autoCompactionSettings: '=',
        validation: '=',
        poolDefault: '=?',
        pools: '=',
        rbac: '=',
        uibModal: '=?'
      },
      isolate: false,
      replace: true,
      templateUrl: 'app/components/directives/mn_buckets_form/mn_buckets_form.html',
      controller: controller
    };

    return mnBucketsForm;

    function threadsEvictionWarning(scope, value) {
      var initialValue = scope.bucketConf[value];
      scope.$watch('bucketConf.' + value, function (newValue) {
        if (initialValue != newValue) {
          scope[value + 'Warning'] = 'Changing ' + (value === 'evictionPolicy' ? 'eviction policy' : 'bucket priority')  +
            ' will restart the bucket. This will lead to closing all open connections and some downtime';
        } else {
          scope[value + 'Warning'] = ''
        }
      });
    }
    function getBucketName($scope) {
      return $scope.bucketConf.isNew ? "." : $scope.bucketConf.name;
    }

    function controller($scope) {
      $scope.goToUsersPage = function () {
        $scope.uibModal.dismiss();
        $state.go("app.admin.security.roles.user");
      };
      $scope.replicaNumberEnabled = $scope.bucketConf.replicaNumber != 0;
      $scope.canChangeBucketsSettings = $scope.bucketConf.isNew;

      if ($scope.rbac && $scope.rbac.cluster.admin.security.read) {
        mnPermissions.getBucketPermissions(getBucketName($scope)).then(function (permissions) {
          var queries = permissions.map(function (permission) {
            return mnUserRolesService.getUsers({permission: permission, pageSize: 4});
          });

          return $q.all(queries).then(function (resps) {
            var uniqUsers = {};
            resps.forEach(function (resp) {
              resp.data.users.forEach(function (user) {
                var name = "";

                if (user.id.length > 16) {
                  name += (user.id.substring(0, 16) + "...");
                } else {
                  name += user.id;
                }
                name += (" (" + (user.domain === "local" ? "couchbase" : user.domain) + ")");

                uniqUsers[user.domain+user.id] = name;
              });
            });

            $scope.users = _.values(uniqUsers);
          });
        });
      }

      $scope.$watch('replicaNumberEnabled', function (isEnabled) {
        if (!isEnabled) {
          $scope.bucketConf.replicaNumber = 0;
          $scope.bucketConf.replicaIndex = 0;
        } else {
          $scope.bucketConf.replicaNumber = Number($scope.bucketConf.replicaNumber) || 1;
        }
      });

      if (!$scope.bucketConf.isNew && !$scope.bucketConf.isWizard) {
        threadsEvictionWarning($scope, 'threadsNumber');
        threadsEvictionWarning($scope, 'evictionPolicy');
      }

      function adaptValidationResult(resp) {
        return mnBucketsDetailsDialogService.adaptValidationResult(resp.data);
      }

      $scope.$watch(function () {
        return {
          bucketConf: $scope.bucketConf,
          autoCompactionSettings: $scope.autoCompactionSettings
        };
      }, function (values) {
        var bucketConf = values.bucketConf;
        var autoCompactionSettings = values.autoCompactionSettings;
        mnPromiseHelper($scope, $http({
          method: 'POST',
          url: bucketConf.uri,
          data: mnBucketsDetailsDialogService.prepareBucketConfigForSaving(bucketConf, autoCompactionSettings, $scope.poolDefault, $scope.pools),
          params: {
            just_validate: 1,
            ignore_warnings: $scope.bucketConf.ignoreWarnings ? 1 : 0
          }
        }))
          .getPromise()
          .then(adaptValidationResult, adaptValidationResult)
          .then(function (result) {
            $scope.validation.result = result;
          });
      }, true);
    }
  }
})();
