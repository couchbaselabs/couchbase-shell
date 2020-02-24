(function () {
  "use strict";

  angular
    .module("mnStatisticsNew")
    .controller("mnStatisticsNewChartBuilderController", mnStatisticsNewChartBuilderController)
    .filter("mnFormatStatsSections", mnFormatStatsSections);

  function mnFormatStatsSections() {
    return function (section) {
      if (section.includes("@")) {
        section = section.substr(1);
      }

      if (section.includes("-")) {
        section = section.substr(0, section.length-1);
      }

      switch (section) {
      case "items": return "Item";
      case "system": return "System";
      case "xdcr": return "XDCR";
      default: return section;
      }
    };
  }


  function mnStatisticsNewChartBuilderController(mnStatisticsNewService, chart, group, scenario, $uibModalInstance, mnStatisticsDescriptionService, $state, mnFormatStatsSectionsFilter, mnFormatServicesFilter, mnStoreService) {
    var vm = this;
    vm.isEditing = !!chart;
    vm.create = create;

    vm.units = {};
    vm.breadcrumbs = {};
    vm.showInPopup = false;
    vm.tabs = ["@system", "@kv", "@index", "@query", "@fts", "@cbas", "@eventing", "@xdcr"];

    vm.statIsNotSupported = [];
    vm.onStatChecked = onStatChecked;
    vm.onSpecificChecked = onSpecificChecked;
    vm.maybeDisableField = maybeDisableField;
    vm.filterStats = filterStats;
    vm.selectTab = selectTab;
    vm.statsDesc = mnStatisticsDescriptionService.stats;
    vm.kvGroups = mnStatisticsDescriptionService.kvGroups;
    vm.getSelectedStats = getSelectedStats;
    vm.getSelectedStatsLength = getSelectedStatsLength;
    vm.formatGroupLabel = formatGroupLabel;
    var selectedUnits = {};
    vm.selectedKVFilters = {};
    var selectedByNodeStats = {};
    var selectedStats = {};

    activate();

    function formatGroupLabel(service) {
      switch (service) {
      case "@index": return "Indexes";
      case "@xdcr": return "Replications";
      case "@kv": return "Views";
      default: return "Items";
      }
    }

    function selectTab(name) {
      vm.tab = name;
    }

    function getSelectedStatsLength() {
      return Object.keys(getSelectedStats()).length;
    }

    function getSelectedStats() {
      return Object
        .keys(vm.newChart.stats)
        .reduce(function (acc, key) {
          if (vm.newChart.stats[key]) {
            acc[key] = vm.newChart.stats[key];
          }
          return acc;
        }, {});
    }

    function reActivateStats() {
      vm.units = {};
      vm.breadcrumbs = {};
      vm.disableStats = false;

      Object.keys(getSelectedStats()).forEach(onStatChecked);
    }

    function filterStats(section) {
      return !section.includes("-");
    }

    function maybeDisableField(descPath) {
      var stat = mnStatisticsNewService.readByPath(descPath);
      return ((vm.newChart.specificStat == "false") &&
              vm.disableStats && !vm.units[stat.unit]) ||
        (vm.newChart.specificStat == "true" &&
         vm.disableStats &&
         !vm.newChart.stats[descPath]);
    }

    function onSpecificChecked() {
      if (vm.newChart.specificStat == "true") {
        selectedStats = vm.newChart.stats;
        vm.newChart.stats = selectedByNodeStats;
      } else {
        selectedByNodeStats = vm.newChart.stats;
        vm.newChart.stats = selectedStats;
      }

      reActivateStats();
    }

    function onStatChecked(descPath) {
      var desc = mnStatisticsNewService.readByPath(descPath);
      var breadcrumb = descPath.split(".");
      if (!desc) {
        vm.newChart.stats[descPath] = false;
        vm.statIsNotSupported.push(breadcrumb.pop());
        return;
      }
      var value = vm.newChart.stats[descPath];

      if (vm.units[desc.unit] === undefined) {
        vm.units[desc.unit] = 0;
      }

      if (value) {
        vm.units[desc.unit] += 1;
        vm.breadcrumbs[breadcrumb
                       .map(mnFormatStatsSectionsFilter)
                       .map(mnFormatServicesFilter)
                       .join(" > ")] = true;
      } else {
        vm.units[desc.unit] -= 1;
        delete vm.breadcrumbs[breadcrumb
                              .map(mnFormatStatsSectionsFilter)
                              .map(mnFormatServicesFilter)
                              .join(" > ")];
      }

      var selectedUnitsCount =
          Object.keys(vm.units).reduce(function (acc, key) {
            if (vm.units[key] > 0) {
              acc += 1
            }
            return acc;
          }, 0);

      if (vm.newChart.specificStat !== "false") {
        vm.disableStats = selectedUnitsCount >= 1;
      } else {
        vm.disableStats = selectedUnitsCount >= 2;
      }
    }

    function activate() {
      if (vm.isEditing) {
        vm.newChart = _.cloneDeep(chart);
        vm.newChart.specificStat = vm.newChart.specificStat.toString();
        vm.selectedGroup = group.id;
        vm.groups = scenario.groups.map(function (id) {
          return mnStoreService.store("groups").get(id);
        });
        Object.keys(vm.newChart.stats).forEach(onStatChecked);
      } else {
        vm.newChart = {
          stats: {},
          size: "small",
          specificStat: "true"
        };
      }

      vm.bucket = $state.params.scenarioBucket;

      if (vm.isEditing) {
        vm.tab = Object.keys(chart.stats).map(function (stat) {
          var tab = stat.split(".")[0];
          if (tab.includes("-")) {
            tab = tab.substr(0, tab.length-1);
          }
          return tab;
        }).sort(function(a, b) {
          return vm.tabs.indexOf(a) - vm.tabs.indexOf(b);
        })[0];

        vm.selectedKVFilters = Object.keys(chart.stats).filter(function (stat) {
          return stat.includes("@kv") && !stat.includes("@items");
        }).reduce(function (acc, kvStat) {
          Object.keys(vm.kvGroups).forEach(function (kvFilter) {
            if (vm.kvGroups[kvFilter].includes(kvStat.split(".")[1])) {
              acc[kvFilter] = true;
            }
          });
          return acc;
        }, {});
      } else {
        vm.tab = vm.tabs[0];
      }
    }

    function create() {
      var chart = {
        size: vm.newChart.size,
        specificStat: vm.newChart.specificStat === "true",
        id: vm.newChart.id,
        stats: getSelectedStats()
      };
      var toGroup = mnStoreService.store("groups").get(vm.selectedGroup || group.id);
      var fromGroup;
      if (vm.isEditing) {
        if (group.id !== vm.selectedGroup) {
          fromGroup = mnStoreService.store("groups").get(group.id);
          fromGroup.charts.splice(fromGroup.charts.indexOf(chart.id), 1);
          toGroup.charts.push(chart.id);
        }
        mnStoreService.store("charts").put(chart);
      } else {
        toGroup.charts.push(mnStoreService.store("charts").add(chart).id);
      }
      $uibModalInstance.close();
    }

  }
})();
