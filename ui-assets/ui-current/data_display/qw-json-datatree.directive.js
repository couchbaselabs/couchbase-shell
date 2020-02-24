/**
 * Angular directive to convert JSON into HTML tree. Inspired by Brian Park's
 * MIT Licensed "angular-json-human.js" which turns JSON to HTML tables.
 *
 *  Extended for trees by Eben Haber at Couchbase.
 *
 *  This class takes a JS object or JSON string, and displays it as an HTML
 *  list, which object members indented. This is similar to pretty-printing
 *  JSON, but is more compact (no braces or commas), and permits using colors
 *  to highlight field names vs. values, and one line from the next.
 *
 */
/* global _, angular */

(function() {

  'use strict';
  angular.module('qwJsonDataTree',[]).directive('qwJsonDataTree', function () {
    return {
      restrict: 'A',
      scope: { data: '=qwJsonDataTree' },
      template: '<div></div>',
      link: function (scope, element) {

        scope.$watch('data', function (json) {
          if (typeof json === 'string') {
            try {
              scope.json_length = json.length;
              json = JSON.parse(json);
            } catch (e) {
            }
          }

          // create an empty div, if we have data, convert it to HTML
          var content = "<div></div>";
          if (json && !_.isEmpty(json))
            content = '<div>' +
              makeHTMLtree(json,"") + "</div>";

          // set the html of the element to what we just generated
          element.html(content);
        });
      }
    };
  });

  // avoid HTML injection by changing tag markers to HTML

  var lt = /</gi;
  var gt = />/gi;
  var mySanitize = function(str) {
    return(str.replace(lt,'&lt;').replace(gt,'&gt;'));
  };

  //recursion in Angular is really inefficient, so we will use a javascript
  //routine to convert the object to an HTML representation. It's not true to
  //the spririt of Angular, but it was taking 10 seconds or more to render
  //a tree of 1000 documents. Even so, browser still can take a few seconds
  //to render a list with tens of thousands of elements


  var makeHTMLtree = function(object,prefix) {
    var result = '';

    // for an object, create an unordered list and iterate over the fields
    if (_.isPlainObject(object)) {

      // mark empty objects so bunches of them don't look weird
      if (_.isEmpty(object)) {
        result += '<span class=ajtd-key>empty object</span>';
        return(result);
      }

      // otherwise iterate over the members
      result += '<ul class="ajtd-type-object">';
      _.forIn(object,function(value,key) {
        // skip the $$hashKey added by angular
        if (key === '$$hashKey') return;
        // for arrays and objects, we need a recursive call
        if (_.isArray(value) || _.isPlainObject(value))
          result += '<li title="' + prefix + mySanitize(key) +
            '"><div><div class=ajtd-key>'
            + mySanitize(key) + '</div><div class=ajtd-object-value>' +
          makeHTMLtree(value,prefix + mySanitize(key) + ".") + '</div></div></li>';
        // otherwise, for primitives, output key/value pair
        else
          result += '<li title="' + prefix + mySanitize(key) +
            '"><table><tr><td class=ajtd-key>' +
            mySanitize(key) + '</td><td class=ajtd-value>' +
            (_.isString(value) ? mySanitize(value) : value) + '</td></tr></table></li>';
      });
      result += "</ul>";
    }

    // for an array, iterate over the elements, and leave out the keys (which are just numbers)

    else if (_.isArray(object)) {
      // if the array is empty, say so
      if (object.length == 0)
          return('<div class=ajtd-key>[]</div>');

      result += '<ul class="ajtd-type-array">';
      for (var i=0; i<object.length; i++) {
        var value = object[i];
        result += '<li  title="' + prefix + "[" + i + "]" +
          '"><div class=ajtd-value>';

        // for arrays and objects, we need a recursive call
        if (_.isArray(value) || _.isPlainObject(value))
          result += makeHTMLtree(value,prefix + "[" + i + "].");

        // otherwise, for primitives, output key/value pair
        else if (_.isString(value))
          result += mySanitize(value);
        // otherwise, for primitives, output key/value pair
        else if (!_.isUndefined(value))
          result += value;
        else
          result += '&nbsp';

        result += '</div></li>';
      }
      result += "</ul>";
    }


    // it's also possible we were passed a primitive value, in which case just put it in a div

    else if (_.isString(object))
      result += '<div class=ajtd-value title="' + prefix + '">' + mySanitize(object) + '</div>';
    else
      result += '<div class=ajtd-value title="' + prefix + '">' + object + '</div>';

      return(result);
  };

})();
