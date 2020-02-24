(function () {
  "use strict";

  angular
    .module('mnWizard')
    .controller('mnTermsAndConditionsController', mnTermsAndConditionsController);

  function mnTermsAndConditionsController($scope, $state, mnWizardService, pools, mnPromiseHelper, mnClusterConfigurationService, mnSettingsClusterService, mnAuthService, mnServersService, mnStatisticsNewService) {
    var vm = this;

    vm.isEnterprise = pools.isEnterprise;
    vm.onSubmit = onSubmit;
    vm.finishWithDefault = finishWithDefault;

    activate();
    function activate() {
      var promise;
      if (vm.isEnterprise) {
        promise = mnWizardService.getEELicense();
      } else {
        promise = mnWizardService.getCELicense();
      }

      mnPromiseHelper(vm, promise)
        .showSpinner()
        .applyToScope("license");

    }

    function finishWithDefault() {
      vm.form.agree.$setValidity('required', !!vm.agree);

      if (vm.form.$invalid) {
        return;
      }

      mnClusterConfigurationService
        .postStats(true).then(function () {
          var services = "kv,index,fts,n1ql";
          if (vm.isEnterprise) {
            services += ",eventing,cbas";
          }
          var setupServicesPromise =
              mnServersService.setupServices({
                services: services,
                setDefaultMemQuotas : true
              });

          mnPromiseHelper(vm, setupServicesPromise)
            .catchErrors()
            .onSuccess(function () {
              var newClusterState = mnWizardService.getNewClusterState();
              mnSettingsClusterService.postIndexSettings({storageMode: vm.isEnterprise ? "plasma" : "forestdb"});
              mnSettingsClusterService
                .postPoolsDefault(false, false, newClusterState.clusterName).then(function () {
                  mnClusterConfigurationService
                    .postAuth(newClusterState.user).then(function () {
                      return mnAuthService
                        .login(newClusterState.user).then(function () {
                          return $state.go('app.admin.overview.statistics');
                        });
                    });
                });
            });
        });
    }

    function onSubmit() {
      vm.form.agree.$setValidity('required', !!vm.agree);

      if (vm.form.$invalid) {
        return;
      }
      $state.go('app.wizard.clusterConfiguration');
    }
  }
})();
