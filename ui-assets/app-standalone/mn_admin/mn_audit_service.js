(function () {
  "use strict";

  angular.module('mnAuditService', [
    'mnPoolDefault'
  ]).factory('mnAuditService', mnAuditServiceFactory);

  function mnAuditServiceFactory($http, $q, mnPoolDefault, IEC) {
    var mnAuditService = {
      getAuditSettings: getAuditSettings,
      saveAuditSettings: saveAuditSettings,
      getAuditDescriptors: getAuditDescriptors,
      getState: getState,
      getNonFilterableDescriptors: getNonFilterableDescriptors
    };

    return mnAuditService;

    function getState() {
      var queries = [
        getAuditSettings()
      ];

      if (mnPoolDefault.export.isEnterprise) {
        if (mnPoolDefault.export.compat.atLeast55) {
          queries.push(getAuditDescriptors());
        }
        if (mnPoolDefault.export.compat.atLeast65) {
          queries.push(getNonFilterableDescriptors())
        }
      }

      return $q.all(queries).then(unpack);
    }

    function getAuditSettings() {
      return $http({
        method: 'GET',
        url: '/settings/audit'
      }).then(function (resp) {
        return resp.data;
      });
    }
    function getNonFilterableDescriptors() {
      return $http({
        method: 'GET',
        url: '/settings/audit/nonFilterableDescriptors'
      }).then(function (resp) {
        return resp.data.map(function (desc) {
          desc.nonFilterable = true;
          desc.enabledByUI = true;
          return desc;
        })
      });
    }
    function getAuditDescriptors() {
      return $http({
        method: 'GET',
        url: '/settings/audit/descriptors'
      }).then(function (resp) {
        return _.clone(resp.data);
      });
    }
    function saveAuditSettings(data, validateOnly) {
      var params = {};
      if (validateOnly) {
        params.just_validate = 1;
      }
      return $http({
        method: 'POST',
        url: '/settings/audit',
        params: params,
        data: pack(data)
      }).then(function (resp) {
        if (resp.data.errors) {
          if (resp.data.errors.disabledUsers) {
            resp.data.errors.disabledUsers =
              resp.data.errors.disabledUsers.replace(/\/local/gi,"/couchbase");
          }
          if (resp.data.errors.rotateSize) {
            resp.data.errors.rotateSize =
              resp.data.errors.rotateSize.replace(/\d+/g, function (bytes) {
                return Number(bytes) / IEC.Mi;
              });
          }
        }
        return resp;
      });
    }
    function mergeEvets(result, value) {
      return result.concat(value);
    }
    function filterDisabled(result, desc) {
      if (!desc.enabledByUI) {
        result.push(desc.id);
      }
      return result;
    }
    function pack(data) {
      var result = {
        auditdEnabled: data.auditdEnabled
      };
      if (mnPoolDefault.export.compat.atLeast55 && mnPoolDefault.export.isEnterprise) {
        result.disabled = _.reduce(
          _.reduce(data.eventsDescriptors, mergeEvets, []),
          filterDisabled, []
        ).join(',');
        result.disabledUsers = data.disabledUsers.replace(/\/couchbase/gi,"/local");
      }
      if (data.auditdEnabled) {
        result.rotateInterval = data.rotateInterval * formatTimeUnit(data.rotateUnit);
        result.logPath = data.logPath;
        result.rotateSize = data.rotateSize;
      }
      if (data.rotateSize) {
        result.rotateSize = data.rotateSize * IEC.Mi;
      }
      return result;
    }
    function formatTimeUnit(unit) {
      switch (unit) {
        case 'minutes': return 60;
        case 'hours': return 3600;
        case 'days': return 86400;
      }
    }
    function unpack(resp) {
      var data = resp[0];
      var eventsDescriptors = resp[1];

      if (data.rotateInterval % 86400 == 0) {
        data.rotateInterval /= 86400;
        data.rotateUnit = 'days';
      } else if (data.rotateInterval % 3600 == 0) {
        data.rotateInterval /= 3600;
        data.rotateUnit = 'hours';
      } else {
        data.rotateInterval /= 60;
        data.rotateUnit = 'minutes';
      }
      if (data.rotateSize) {
        data.rotateSize = data.rotateSize / IEC.Mi;
      }
      if (mnPoolDefault.export.isEnterprise) {
        if (mnPoolDefault.export.compat.atLeast55) {
          var mapDisabledIDs = _.groupBy(data.disabled);
          eventsDescriptors.forEach(function (desc) {
            desc.enabledByUI = !mapDisabledIDs[desc.id];
          });
          if (mnPoolDefault.export.compat.atLeast65) {
            Array.prototype.push.apply(eventsDescriptors, resp[2]);
          }
          data.eventsDescriptors = _.groupBy(eventsDescriptors, "module");
          data.disabledUsers = data.disabledUsers.map(function (user) {
            return user.name + "/" + (user.domain === "local" ? "couchbase" : user.domain);
          }).join(',');
        }
      }
      data.logPath = data.logPath || "";
      return data;
    }
  }
})();
