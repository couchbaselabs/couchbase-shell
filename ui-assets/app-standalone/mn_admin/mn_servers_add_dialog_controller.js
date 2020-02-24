(function () {
  "use strict";

  angular
    .module('mnServers')
    .controller('mnServersAddDialogController', mnServersAddDialogController)

  function mnServersAddDialogController($scope, $rootScope, $q, $uibModal, mnServersService, $uibModalInstance, mnHelper, mnPromiseHelper, groups, mnClusterConfigurationService, mnPoolDefault, mnRootCertificateService) {
    var vm = this;

    vm.specifyDisk = false;

    vm.addNodeConfig = {
      services: {
        model: {
          kv: true,
          index: true,
          n1ql: true,
          fts: true
        }
      },
      credentials: {
        hostname: '',
        user: 'Administrator',
        password: ''
      }
    };
    if ($scope.poolDefault.isEnterprise) {
      vm.addNodeConfig.services.model.cbas = true;
      vm.addNodeConfig.services.model.eventing = true;
    }
    vm.isGroupsAvailable = !!groups;
    vm.onSubmit = onSubmit;

    if (vm.isGroupsAvailable) {
      vm.addNodeConfig.selectedGroup = groups.groups[0];
      vm.groups = groups.groups;
    }

    activate();

    function activate() {
      reset();
      if ($scope.poolDefault.isEnterprise) {
        mnPromiseHelper(vm, mnRootCertificateService.getDefaultCertificate())
          .applyToScope("certificate");
      }
      mnClusterConfigurationService.getSelfConfig().then(function (selfConfig) {
        var rv = {};
        rv.selfConfig = selfConfig;
        if ($scope.poolDefault.isEnterprise) {
          rv.cbasDirs = selfConfig.storage.hdd[0].cbas_dirs;
        }
        rv.dbPath = selfConfig.storage.hdd[0].path;
        rv.indexPath = selfConfig.storage.hdd[0].index_path;
        rv.eventingPath = selfConfig.storage.hdd[0].eventing_path;
        vm.selfConfig = rv;
      });
    }
    function postDiskStorage(resp) {
      if (resp && resp.data) {
        vm.optNode = resp.data.otpNode;
      }
      var data = {
        path: vm.selfConfig.dbPath,
        index_path: vm.selfConfig.indexPath
      };
      data.eventing_path = vm.selfConfig.eventingPath;
      if ($scope.poolDefault.isEnterprise) {
        data.cbas_path = vm.selfConfig.cbasDirs;
      }
      var promise = mnClusterConfigurationService.postDiskStorage(data, vm.optNode);
      return mnPromiseHelper(vm, promise)
        .catchErrors('postDiskStorageErrors')
        .getPromise();
    }
    function reset() {
      vm.focusMe = true;
    }
    function onSubmit(form) {
      if (vm.viewLoading) {
        return;
      }

      var servicesList = mnHelper.checkboxesToList(vm.addNodeConfig.services.model);

      form.$setValidity('services', !!servicesList.length);

      if (form.$invalid) {
        return reset();
      }
      var promise;
      if (vm.postDiskStorageErrors) {
        if (vm.specifyDisk) {
          promise = postDiskStorage();
        } else {
          $uibModalInstance.close();
        }
      } else {
        promise = mnServersService
          .addServer(vm.addNodeConfig.selectedGroup,
                     vm.addNodeConfig.credentials,
                     servicesList);
        if (vm.specifyDisk) {
          promise = promise.then(postDiskStorage);
        }
      }

      mnPromiseHelper(vm, promise, $uibModalInstance)
        .showGlobalSpinner()
        .catchErrors()
        .closeOnSuccess()
        .broadcast("reloadServersPoller")
        .broadcast("maybeShowMemoryQuotaDialog", vm.addNodeConfig.services.model)
        .showGlobalSuccess("Server added successfully!");
    };
  }
})();
