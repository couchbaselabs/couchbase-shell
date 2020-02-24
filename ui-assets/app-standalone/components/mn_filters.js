(function () {
  "use strict";

  angular
    .module('mnFilters', [])
    .filter('mnCount', mnCount)
    .filter('removeEmptyValue', removeEmptyValue)
    .filter('formatProgressMessage', formatProgressMessage)
    .filter('mnCloneOnlyData', mnCloneOnlyData)
    .filter('$httpParamSerializerJQLike', httpParamSerializerJQLike)
    .filter('mnParseHttpDate', mnParseHttpDate)
    .filter('mnPrepareQuantity', mnPrepareQuantity)
    .filter('mnCalculatePercent', mnCalculatePercent)
    .filter('mnEllipsisiseOnLeft', mnEllipsisiseOnLeft)
    .filter('mnRescaleForSum', mnRescaleForSum)
    .filter('mnNaturalSorting', mnNaturalSorting)
    .filter('mnMakeSafeForCSS', mnMakeSafeForCSS)
    .filter('mnStripPortHTML', mnStripPortHTML)
    .filter('mnTruncateTo3Digits', mnTruncateTo3Digits)
    .filter('mnFormatQuantity', mnFormatQuantity)
    .filter('mnFormatMemSize', mnFormatMemSize)
    .filter('mnFormatUptime', mnFormatUptime)
    .filter('mnMBtoBytes', mnMBtoBytes)
    .filter('mnBytesToMB', mnBytesToMB)
    .filter('parseVersion', parseVersion)
    .filter('getStringBytes', getStringBytes)
    .filter('mnFormatServices', mnFormatServices)
    .filter('mnFormatServicesArray', mnFormatServicesArray)
    .filter('mnPrettyVersion', mnPrettyVersion)
    .filter('encodeURIComponent', encodeURIComponentFilter)
    .filter('mnTrustAsHtml', mnTrustAsHtml)
    .filter('mnMath', mnMath)
    .filter('lodash', lodash)
    .filter('isDisabled', isDisabled)
    .filter('mnIntegerToString', mnIntegerToString)
    .filter('mnFormatStorageMode', mnFormatStorageMode)
    .filter('mnLimitTo', mnLimitTo)
    .filter('jQueryLikeParamSerializer', jQueryLikeParamSerializer)
    .filter('decodeCompatVersion', decodeCompatVersion)
    .filter('mnMsToTime', mnMsToTime)
    .filter('mnServersListFilter', mnServersListFilter)
    .filter("formatFailoverWarnings", formatFailoverWarnings);



  function formatFailoverWarnings() {
    return function (warning) {
      switch (warning) {
      case 'rebalanceNeeded':
        return 'Rebalance required, some data is not currently replicated.';
      case 'hardNodesNeeded':
        return 'At least two servers with the data service are required to provide replication.';
      case 'softNodesNeeded':
        return 'Additional active servers required to provide the desired number of replicas.';
      case 'softRebalanceNeeded':
        return 'Rebalance recommended, some data does not have the desired replicas configuration.';
      default: return warning;
      }
    };
  }



  //filter by multiple strictly defined fields in the node
  //e.g filterField -> "apple 0.0.0.0 data"
  //will return all kv nodes which run on macos 0.0.0.0 host
  function mnServersListFilter($filter, mnFormatServicesFilter) {
    return function (nodes, searchValue, groupsByHostname) {
      return $filter('filter')(nodes, function (node) {
        if (searchValue === "" || searchValue === undefined) {
          return true;
        }

        var interestingFields = ["hostname", "status"];
        var l2 = interestingFields.length;
        var l3 = node.services.length;
        var i2;
        var i3;
        var searchFiled;
        searchValue = searchValue.toLowerCase();
        var rv = false;

        //look in services
        if ($filter('orderBy')(node.services.map(function (node) {
          return mnFormatServicesFilter(node).toLowerCase();
        })).join(" ").indexOf(searchValue) > -1) {
          rv = true;
        }

        //look in interestingFields
        loop2:
        for (i2 = 0; i2 < l2; i2++) {
          searchFiled = interestingFields[i2];
          if (node[searchFiled].toLowerCase().indexOf(searchValue) > -1) {
            rv = true;
            break loop2;
          }
        }

        //look in group name
        if (!rv && groupsByHostname && groupsByHostname[node.hostname] &&
            groupsByHostname[node.hostname].name.toLowerCase().indexOf(searchValue) > -1) {
          rv = true;
        }

        return rv;
      });
    }
  }

  function decodeCompatVersion() {
    return function (version) {
      var major = Math.floor(version / 0x10000);
      var minor = version - (major * 0x10000);
      return major.toString() + "." + minor.toString();
    }
  }

  //function is borrowed from the Angular source code because we want to
  //use $httpParamSerializerJQLik but with properly encoded params via
  //encodeURIComponent since it uses correct application/x-www-form-urlencoded
  //encoding algorithm, in accordance with
  //https://www.w3.org/TR/html5/forms.html#url-encoded-form-data
  function jQueryLikeParamSerializer() {
    return function (params) {
      if (!params) {
        return '';
      }
      var parts = [];
      serialize(params, '', true);
      return parts.join('&');

      function serializeValue(v) {
        if (angular.isObject(v)) {
          return angular.isDate(v) ? v.toISOString() : angular.toJson(v);
        }
        if (v === null || angular.isUndefined(v)) {
          return "";
        }
        return v;
      }

      function serialize(toSerialize, prefix, topLevel) {
        if (angular.isArray(toSerialize)) {
          angular.forEach(toSerialize, function (value, index) {
            serialize(value, prefix + (angular.isObject(value) ? '[' + index + ']' : ''));
          });
        } else if (angular.isObject(toSerialize) && !angular.isDate(toSerialize)) {
          angular.forEach(toSerialize, function (value, key) {
            serialize(value, prefix +
                      (topLevel ? '' : '[') +
                      key +
                      (topLevel ? '' : ']'));
          });
        } else {
          parts.push(encodeURIComponent(prefix) + '=' + encodeURIComponent(serializeValue(toSerialize)));
        }
      }
    }
  }

  //angular limitTo uses slice in order to truncate sting
  //the method is very slow in case string is very big
  //therefore we use the substring method here
  function mnLimitTo() {
    return function (string, limit) {
      if (string === undefined) {
        return "";
      }
      return (angular.isString(string) ? string : angular.toJson(string)).substring(0, limit);
    }
  }
  function mnFormatStorageMode() {
    return function (value, isEnterprise) {
      switch (value) {
      case "plasma": return "Standard GSI";
      case "forestdb": return (isEnterprise ? "Legacy" : "Standard") + " GSI";
      case "memory_optimized": return "Memory Optimized GSI";
      default: return value;
      }
    };
  }

  function lodash() {
    return function (/*lodash medthod and args..*/) {
      var args = Array.prototype.slice.call(arguments, 0);
      var method = args.shift();
      return _[method].apply(_, args);
    };
  }

  function isDisabled() {
    //TODO implement this across entire app
    return function ($event) {
      return angular.element($event.currentTarget).hasClass("dynamic_disabled");
    };
  }

  function mnMath() {
    return function () {
      var args = Array.prototype.slice.call(arguments, 0);
      var method = args.shift();
      return Math[method].apply(null, args);
    }
  }

  function mnTrustAsHtml($sce) {
    return function (html) {
      return $sce.trustAsHtml(html);
    };
  }
  var basedigits = "0123456789ABCDEF";
  function mnIntegerToString() {
    return function (number, base) {
      var rv = [];
      var sign = '';
      if (number < 0) {
        sign = '-';
        number = -number;
      }
      do {
        var r = number % base;
        number = (number / base) >> 0;
        rv.push(basedigits.charAt(r));
      } while (number != 0);
      rv.push(sign);
      rv.reverse();
      return rv.join('');
    }
  }

  function mnCount() {
    return function (count, text) {
      if (count == null) {
        return '?' + text + '(s)';
      }
      count = Number(count);
      if (count > 1) {
        var lastWord = text.split(/\s+/).slice(-1)[0];
        var specialPluralizations = {
          'copy': 'copies'
        };
        var specialCase = specialPluralizations[lastWord];
        if (specialCase) {
          text = specialCase;
        } else {
          text += 's';
        }
      }
      return [String(count), ' ', text].join('');
    };
  }
  function removeEmptyValue() {
    return function (object) {
      return _.transform(_.clone(object), function (result, n, key) {
        if (n === "") {
          return;
        }
        result[key] = n;
      });
    };
  }
  function addNodeCount(perNode) {
    var serversCount = (_.keys(perNode) || []).length;
    return serversCount + " " + (serversCount === 1 ? 'node' : 'nodes');
  }
  function formatProgressMessage() {
    return function (task, includeRebalance) {
      switch (task.type) {
      case "indexer":
        return "building view index " + task.bucket + "/" + task.designDocument;
      case "global_indexes":
        return "building index " + task.index  + " on bucket " + task.bucket;
      case "view_compaction":
        return "compacting view index " + task.bucket + "/" + task.designDocument;
      case "bucket_compaction":
        return "compacting bucket " + task.bucket;
      case "loadingSampleBucket":
        return "loading sample: " + task.bucket;
      case "orphanBucket":
        return "orphan bucket: " + task.bucket;
      case "clusterLogsCollection":
        return "collecting logs from " + addNodeCount(task.perNode);
      case "rebalance":
        return !!includeRebalance;
      }
    };
  }
  function mnCloneOnlyData() {
    return function (data) {
      return JSON.parse(JSON.stringify(data));
    };
  }
  function httpParamSerializerJQLike($httpParamSerializerJQLike) {
    return $httpParamSerializerJQLike;
  }
  function mnParseHttpDate() {
    var rfc1123RE = /^\s*[a-zA-Z]+, ([0-9][0-9]) ([a-zA-Z]+) ([0-9]{4,4}) ([0-9]{2,2}):([0-9]{2,2}):([0-9]{2,2}) GMT\s*$/m;
    var rfc850RE = /^\s*[a-zA-Z]+, ([0-9][0-9])-([a-zA-Z]+)-([0-9]{2,2}) ([0-9]{2,2}):([0-9]{2,2}):([0-9]{2,2}) GMT\s*$/m;
    var asctimeRE = /^\s*[a-zA-Z]+ ([a-zA-Z]+) ((?:[0-9]| )[0-9]) ([0-9]{2,2}):([0-9]{2,2}):([0-9]{2,2}) ([0-9]{4,4})\s*$/m;

    var monthDict = {};

    (function () {
      var monthNames = ["January", "February", "March", "April", "May", "June",
                        "July", "August", "September", "October", "November", "December"];

      for (var i = monthNames.length-1; i >= 0; i--) {
        var name = monthNames[i];
        var shortName = name.substring(0, 3);
        monthDict[name] = i;
        monthDict[shortName] = i;
      }
    })();

    var badDateException;
    (function () {
      try {
        throw {};
      } catch (e) {
        badDateException = e;
      }
    })();
    function parseMonth(month) {
      var number = monthDict[month];
      if (number === undefined)
        throw badDateException;
      return number;
    }
    function doParseHTTPDate(date) {
      var match;
      if ((match = rfc1123RE.exec(date)) || (match = rfc850RE.exec(date))) {
        var day = parseInt(match[1], 10);
        var month = parseMonth(match[2]);
        var year = parseInt(match[3], 10);

        var hour = parseInt(match[4], 10);
        var minute = parseInt(match[5], 10);
        var second = parseInt(match[6], 10);

        return new Date(Date.UTC(year, month, day, hour, minute, second));
      } else if ((match = asctimeRE.exec(date))) {
        var month = parseMonth(match[1]);
        var day = parseInt(match[2], 10);

        var hour = parseInt(match[3], 10);
        var minute = parseInt(match[4], 10);
        var second = parseInt(match[5], 10);

        var year = parseInt(match[6], 10);

        return new Date(Date.UTC(year, month, day, hour, minute, second));
      } else {
        throw badDateException;
      }
    }

    return function (date, badDate) {
      try {
        return doParseHTTPDate(date);
      } catch (e) {
        if (e === badDateException) {
          return badDate || (new Date());
        }
        throw e;
      }
    }
  }
  function mnPrepareQuantity() {
    return function (value, K) {
      K = K || 1024;

      var M = K*K;
      var G = M*K;
      var T = G*K;

      if (K !== 1024 && K !== 1000) {
        throw new Error("Unknown number system");
      }

      var t = _.detect([[T,'T'],[G,'G'],[M,'M'],[K,'K']], function (t) {
        return value >= t[0];
      }) || [1, ''];

      if (K === 1024) {
        t[1] += 'B';
      }

      return t;
    };
  }
  function mnCalculatePercent() {
    return function (value, total) {
      return (value * 100 / total) >> 0;
    };
  }
  function mnEllipsisiseOnLeft() {
    return function (text, length) {
      if (length <= 3) {
        // asking for stupidly short length will cause this to do
        // nothing
        return text;
      }
      if (text.length > length) {
        return "..." + text.slice(3-length);
      }
      return text;
    };
  }
  function mnRescaleForSum() {
    // proportionaly rescales values so that their sum is equal to given
    // number. Output values need to be integers. This particular
    // algorithm tries to minimize total rounding error. The basic approach
    // is same as in Brasenham line/circle drawing algorithm.
    return function (newSum, values, oldSum) {
      if (oldSum == null) {
        oldSum = _.inject(values, function (a,v) {return a+v;}, 0);
      }
      // every value needs to be multiplied by newSum / oldSum
      var error = 0;
      var outputValues = new Array(values.length);
      for (var i = 0; i < outputValues.length; i++) {
        var v = values[i];
        v *= newSum;
        v += error;
        error = v % oldSum;
        outputValues[i] = Math.floor(v / oldSum);
      }
      return outputValues;
    };
  }
  function mnNaturalSorting() {
     /*
     * Natural Sort algorithm for Javascript - Version 0.6 - Released under MIT license
     * Author: Jim Palmer (based on chunking idea from Dave Koelle)
     * Contributors: Mike Grier (mgrier.com), Clint Priest, Kyle Adams, guillermo
     *
     * Alterations: removed date and hex parsing/sorting
     */
    return function naturalSort(a, b) {
      var re = /(^-?[0-9]+(\.?[0-9]*)[df]?e?[0-9]?$|^0x[0-9a-f]+$|[0-9]+)/gi,
        sre = /(^[ ]*|[ ]*$)/g,
        ore = /^0/,
        // convert all to strings and trim()
        x = a.toString().replace(sre, '') || '',
        y = b.toString().replace(sre, '') || '',
        // chunk/tokenize
        xN = x.replace(re, '\0$1\0').replace(/\0$/,'').replace(/^\0/,'').split('\0'),
        yN = y.replace(re, '\0$1\0').replace(/\0$/,'').replace(/^\0/,'').split('\0');
      // natural sorting through split numeric strings and default strings
      for(var cLoc=0, numS=Math.max(xN.length, yN.length); cLoc < numS; cLoc++) {
        // find floats not starting with '0', string or 0 if not defined (Clint Priest)
        var oFxNcL = !(xN[cLoc] || '').match(ore) && parseFloat(xN[cLoc]) || xN[cLoc] || 0;
        var oFyNcL = !(yN[cLoc] || '').match(ore) && parseFloat(yN[cLoc]) || yN[cLoc] || 0;
        // handle numeric vs string comparison - number < string - (Kyle Adams)
        if (isNaN(oFxNcL) !== isNaN(oFyNcL)) return (isNaN(oFxNcL)) ? 1 : -1;
        // rely on string comparison if different types - i.e. '02' < 2 != '02' < '2'
        else if (typeof oFxNcL !== typeof oFyNcL) {
          oFxNcL += '';
          oFyNcL += '';
        }
        if (oFxNcL < oFyNcL) return -1;
        if (oFxNcL > oFyNcL) return 1;
      }
      return 0;
    };
  }
  function mnMakeSafeForCSS() {
    return function (name) {
      return name.replace(/[^a-z0-9]/g, function (s) {
        var c = s.charCodeAt(0);
        if (c == 32) return '-';
        if (c >= 65 && c <= 90) return '_' + s.toLowerCase();
        return '__' + ('000' + c.toString(16)).slice(-4);
      });
    };
  }
  function mnStripPortHTML() {
    var cachedAllServers;
    var cachedIsStripping;
    var strippingRE = /:8091$/;

    return function (value, allServers) {
      if (allServers === undefined) {
        throw new Error("second argument is required!");
      }
      if (cachedAllServers === allServers) {
        var isStripping = cachedIsStripping;
      } else {
        if (allServers.length == 0 || _.isString(allServers[0])) {
          var allNames = allServers;
        } else {
          var allNames = _.pluck(allServers, 'hostname');
        }
        var isStripping = _.all(allNames, function (h) {return h.match(strippingRE);});
        cachedIsStripping = isStripping;
        cachedAllServers = allServers;
      }
      if (isStripping) {
        var match = value.match(strippingRE);
        return match ? value.slice(0, match.index) : value;
      }
      return value;
    };
  }
  function mnTruncateTo3Digits() {
    return function (value, leastScale, roundMethod) {
      if (!value) {
        return 0;
      }
      var scale = _.detect([100, 10, 1, 0.1, 0.01, 0.001], function (v) {return value >= v;}) || 0.0001;
      if (leastScale != undefined && leastScale > scale) {
        scale = leastScale;
      }
      scale = 100 / scale;
      return Math[roundMethod || "round"](value*scale)/scale;
    };
  }
  function mnFormatQuantity(mnPrepareQuantityFilter, mnTruncateTo3DigitsFilter) {
    return function (value, numberSystem, spacing) {
      if (!value && !_.isNumber(value)) {
        return value;
      }
      if (spacing == null) {
        spacing = '';
      }
      if (numberSystem === 1000 && value <= 9999 && value % 1 === 0) { // MB-11784
        return value;
      }

      var t = mnPrepareQuantityFilter(value, numberSystem);
      return [mnTruncateTo3DigitsFilter(value/t[0], undefined, "floor"), spacing, t[1]].join('');
    };
  }
  function mnFormatMemSize(mnFormatQuantityFilter) {
    return function (value) {
      return mnFormatQuantityFilter(value, null, ' ');
    };
  }
  function mnMsToTime() {
    return function (ms) {
      var d, h, m, s;
      s = Math.floor(ms / 1000);
      m = Math.floor(s / 60);
      s = s % 60;
      h = Math.floor(m / 60);
      m = m % 60;
      d = Math.floor(h / 24);
      h = h % 24;

      return (h ? (h + ':') : '') +
        ((m > 9) ? m : ("0" + m)) + ":"  + ((s > 9) ? s : ("0" + s));
    }
  }
  function mnFormatUptime() {
    return function (seconds, precision) {
      precision = precision || 8;

      var arr = [[86400, "days", "day"],
                 [3600, "hours", "hour"],
                 [60, "minutes", "minute"],
                 [1, "seconds", "second"]];

      var rv = [];

      _.each(arr, function (item) {
        var period = item[0];
        var value = (seconds / period) >> 0;
        seconds -= value * period;
        if (value) {
          rv.push(String(value) + ' ' + (value > 1 ? item[1] : item[2]));
        }
        return !!--precision;
      });
      return rv.join(', ');
    };
  }
  function mnMBtoBytes(IEC) {
    return function (MB) {
      return MB * IEC.Mi;
    };
  }
  function mnBytesToMB(IEC) {
    return function (bytes) {
      return Math.floor(bytes / IEC.Mi);
    };
  }
  function parseVersion() {
    return function (str) {
      if (!str) {
        return;
      }
      // Expected string format:
      //   {release version}-{build #}-{Release type or SHA}-{enterprise / community}
      // Example: "1.8.0-9-ga083a1e-enterprise"
      var a = str.split(/[-_]/);
      if (a.length === 3) {
        // Example: "1.8.0-9-enterprise"
        //   {release version}-{build #}-{enterprise / community}
        a.splice(2, 0, undefined);
      }
      a[0] = (a[0].match(/[0-9]+\.[0-9]+\.[0-9]+/) || ["0.0.0"])[0];
      a[1] = a[1] || "0";
      // a[2] = a[2] || "unknown";
      // We append the build # to the release version when we display in the UI so that
      // customers think of the build # as a descriptive piece of the version they're
      // running (which in the case of maintenance packs and one-off's, it is.)
      a[3] = (a[3] && (a[3].substr(0, 1).toUpperCase() + a[3].substr(1))) || "DEV";
      return a; // Example result: ["1.8.0-9", "9", "ga083a1e", "Enterprise"]
    }
  }
  function getStringBytes() {
    return function (countMe) {
      if (!_.isString(countMe)) {
        return 0;
      }
      var escapedStr = encodeURI(countMe);
      var escapedStrLength = escapedStr.length;

      if (escapedStr.indexOf("%") != -1) {
        var count = escapedStr.split("%").length - 1 || 1;
        return count + (escapedStrLength - (count * 3));
      } else {
        return escapedStrLength;
      }
    }
  }
  function mnFormatServices() {
    return function (service) {
      switch (service) {
      case 'kv': return 'Data';
      case 'query':
      case 'n1ql': return 'Query';
      case 'index': return 'Index';
      case 'fts': return 'Search';
      case 'eventing': return 'Eventing';
      case 'cbas': return 'Analytics';
      default: return service;
      }
    }
  }
  function mnFormatServicesArray() {
    return function (services) {
      return _.map(services, mnFormatServices());
    };
  }
  function mnPrettyVersion(parseVersionFilter) {

    return function (str, full) {
      if (!str) {
        return;
      }
      var a = parseVersionFilter(str);
      // Example default result: "Enterprise Edition 1.8.0-7  build 7"
      // Example full result: "Enterprise Edition 1.8.0-7  build 7-g35c9cdd"
      var suffix = "";
      if (full && a[2]) {
        suffix = '-' + a[2];
      }
      return [a[3], "Edition", a[0], "build",  a[1] + suffix].join(' ');
    };
  }
  function encodeURIComponentFilter() {
    return encodeURIComponent;
  }
})();
