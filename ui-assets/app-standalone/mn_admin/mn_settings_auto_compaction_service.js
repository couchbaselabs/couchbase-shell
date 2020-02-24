(function () {
  "use strict";

  angular.module('mnSettingsAutoCompactionService', [
    'mnFilters',
    'mnHelper'
  ]).factory('mnSettingsAutoCompactionService', mnSettingsAutoCompactionServiceFactory);

  function mnSettingsAutoCompactionServiceFactory($http, $q, mnBytesToMBFilter, mnMBtoBytesFilter, mnHelper) {
    var mnSettingsAutoCompactionService = {
      prepareSettingsForView: prepareSettingsForView,
      prepareSettingsForSaving: prepareSettingsForSaving,
      getAutoCompaction: getAutoCompaction,
      saveAutoCompaction: saveAutoCompaction,
      prepareErrorsForView: prepareErrorsForView
    };

    return mnSettingsAutoCompactionService;

    function prepareValuesForView(holder) {
      angular.forEach(['size', 'percentage'], function (fieldName) {
        if (holder[fieldName] === "undefined") {
          holder[fieldName] = "";
        } else {
          holder[fieldName + 'Flag'] = true;
          fieldName === "size" && (holder[fieldName] = mnBytesToMBFilter(holder[fieldName]));
        }
      });
    }
    function prepareSettingsForView(settings, isBucketsDetails) {
      var acSettings = settings.autoCompactionSettings;
      prepareValuesForView(acSettings.databaseFragmentationThreshold);
      prepareValuesForView(acSettings.viewFragmentationThreshold);
      if (isBucketsDetails) {
        delete acSettings.indexFragmentationThreshold;
        delete acSettings.indexCircularCompaction;
      } else {
        if (acSettings.indexCircularCompaction) {
          acSettings.indexCircularCompactionFlag = acSettings.indexCompactionMode === "circular";
          if (acSettings.indexCircularCompaction.daysOfWeek == "") {
            acSettings.indexCircularCompactionDaysOfWeek = {};
          } else {
            acSettings.indexCircularCompactionDaysOfWeek = mnHelper.listToCheckboxes(acSettings.indexCircularCompaction.daysOfWeek.split(","));
          }
          acSettings.indexCircularCompaction = acSettings.indexCircularCompaction.interval;
        }
      }
      if (acSettings.indexFragmentationThreshold) {
        prepareValuesForView(acSettings.indexFragmentationThreshold);
      }
      acSettings.allowedTimePeriodFlag = !!acSettings.allowedTimePeriod;
      acSettings.purgeInterval = settings.purgeInterval;
      !acSettings.allowedTimePeriod && (acSettings.allowedTimePeriod = {
        abortOutside: false,
        toMinute: null,
        toHour: null,
        fromMinute: null,
        fromHour: null
      });
      return acSettings;
    }
    function prepareVluesForSaving(holder) {
      angular.forEach(['size', 'percentage'], function (fieldName) {
        if (!holder[fieldName + 'Flag']) {
          delete holder[fieldName];
        } else {
          fieldName === "size" && (holder[fieldName] = mnMBtoBytesFilter(holder[fieldName]));
          fieldName === "percentage" && (holder[fieldName] = Number(holder[fieldName]));
        }
      });
    }
    function prepareSettingsForSaving(acSettings) {
      if (!acSettings) {
        return acSettings;
      }

      acSettings = _.clone(acSettings, true);

      acSettings.purgeInterval = Number(acSettings.purgeInterval);

      if (!acSettings.allowedTimePeriodFlag) {
        delete acSettings.allowedTimePeriod;
      }
      if (acSettings.indexCircularCompaction) {
        acSettings.indexCompactionMode = acSettings.indexCircularCompactionFlag === true ? "circular" : "full";
        acSettings.indexCircularCompaction = {
          daysOfWeek: mnHelper.checkboxesToList(acSettings.indexCircularCompactionDaysOfWeek).join(','),
          interval: acSettings.indexCircularCompaction
        };
        delete acSettings.indexCircularCompactionFlag;
        delete acSettings.indexCircularCompactionDaysOfWeek;
      }
      prepareVluesForSaving(acSettings.databaseFragmentationThreshold);
      prepareVluesForSaving(acSettings.viewFragmentationThreshold);
      if (acSettings.indexFragmentationThreshold) {
        prepareVluesForSaving(acSettings.indexFragmentationThreshold);
        delete acSettings.indexFragmentationThreshold.sizeFlag;
        delete acSettings.indexFragmentationThreshold.percentageFlag;
      }
      delete acSettings.databaseFragmentationThreshold.sizeFlag;
      delete acSettings.viewFragmentationThreshold.percentageFlag;
      delete acSettings.viewFragmentationThreshold.sizeFlag;
      delete acSettings.databaseFragmentationThreshold.percentageFlag;
      delete acSettings.allowedTimePeriodFlag;
      return acSettings;
    }
    function getAutoCompaction(isBucketsDetails) {
      return $http.get('/settings/autoCompaction').then(function (resp) {
        return mnSettingsAutoCompactionService.prepareSettingsForView(resp.data, isBucketsDetails);
      });
    }
    function prepareErrorsForView(errors) {
      angular.forEach(["fromHour", "fromMinute", "toHour", "toMinute"], function (value) {
        if (errors["indexCircularCompaction[interval]["+value+"]"]) {
          errors["indexCircularCompaction["+value+"]"] = errors["indexCircularCompaction[interval]["+value+"]"];
          delete errors["indexCircularCompaction[interval]["+value+"]"];
        }
      });
      angular.forEach(errors, function (value, key) {
        errors[key.replace('[', '_').replace(']', '_')] = value;
      });
      return errors;
    }
    function saveAutoCompaction(autoCompactionSettings, params) {
      return $http({
        method: 'POST',
        url: '/controller/setAutoCompaction',
        params: params || {},
        data: mnSettingsAutoCompactionService.prepareSettingsForSaving(autoCompactionSettings)
      }).then(null, function (resp) {
        if (resp.data) {
          resp.data.errors = prepareErrorsForView(resp.data.errors);
        }
        return $q.reject(resp);
      });
    }
  }
})();
