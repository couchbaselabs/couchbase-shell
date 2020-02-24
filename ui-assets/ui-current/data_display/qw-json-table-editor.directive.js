/**
 * Make a table of editable cells, each row corresponding to a document, each
 * with a meta-id that we want to show as well.
 *
 * This is a version of the qwJsonTableEditor but customized to deal with the
 * output of one particular query:
 *
 *  select meta().id, * from <bucket name> data [optional where, limit clauses]
 *
 *  As a result, we expect an array of objects, each of which has a 'data' field
 *  (pointing to the document contents) and an 'id' field, which contains the
 *  meta_id. If the user makes changes and clicks the "update" button, the
 *  meta_id is used to update the appropriate document in Couchbase.
 *
 */
/* global _, angular */
(function() {

  'use strict';
  angular.module('qwJsonTableEditor', [])
  .directive('qwJsonTableEditor', ['$compile','$timeout','mnPermissions',getTableEditor]);

  function getTableEditor($compile,$timeout,mnPermissions) {
    return {
      restrict: 'E',
//    scope: { data: '=qwJsonTableEditor' },
      scope: { data: '=data', controller: '=controller'},
      template: '<div></div>',
      link: function (scope, element) {

        scope.$watch('data', buildEditorFromJson);
        scope.rbac = mnPermissions.export;
        scope.getTooltip = getTooltip;

        function buildEditorFromJson(json) {
          // start by putting up a message
          element.append(angular.element('<div class="text-medium">Rendering results...</div>'));

          $timeout(function() {createEditorFromJson(json)},10); // let the above message render, then build
        }
        function createEditorFromJson(json) {

          // start with an empty div, if we have data convert it to HTML
          var wrapper = '<div class="data-table-wrapper">{}</div>';
          var table;
          var warning;
          htmlElement = element;
          if (!json)
            json = tdata;

          var content = "<div>{}</div>";

          // do we have data to work with?
          if (json && _.isArray(json)) {
            tdata = json;
            scope.results = json;
            scope.dec = scope.controller;
            meta = getMetaData(json);

            // make the table header with the top-level fields

            if (meta.truncated)
              warning = angular.element('<div class="error text-small" style="margin-bottom:-20px">Some documents too large for tabular editing, tabular view truncated.</div>`');
            header = angular.element(createHTMLheader(meta,scope.controller));
            wrapper = '<div class="data-table-wrapper show-scrollbar"></div>';

            var tableHTML = makeHTMLTopLevel();
            table = angular.element(tableHTML);
          }

          //
          // otherwise show error message
          //

          else {
            wrapper = '<div class="data-table-wrapper">' + json + '</div>';
            header = null;
            table = null;
          }

          // even if the json was empty, we have a wrapper element
          wrapperElement = angular.element(wrapper);
          if (table) {
            //wrapperElement.append(table);
            $compile(table)(scope, function(compiledTable) {wrapperElement.append(compiledTable)}); // need to compile to link generated HTML with angular
          }

          // clear out our element. If we have a header add it, then add the wrapper
          htmlElement.empty();
          if (warning)
            htmlElement.append(warning);
          if (header) {
            $compile(header)(scope, function(header) {

              htmlElement.append(header);
              // sync scrolling between the header and the main table
              // listen on scrolling in the data window
              wrapperElement[0].addEventListener("scroll",function() {
                if (header) {
                  header[0].scrollLeft = wrapperElement[0].scrollLeft;
                }
              });
              // also listen on horizontal scrolling in the header, to keep the data in sync
              if (header) header[0].addEventListener("scroll",function() {
                wrapperElement[0].scrollLeft = header[0].scrollLeft
              });

              // put a sort listener on the data columns
              var startSortColumn = meta.hasNonObject ? 3 : 2; // don't allow sorting on first few columns
              // allow sorting by id
              header[0].childNodes[startSortColumn - 1].addEventListener("click",function() {
                sortTable(this,scope,$compile,$timeout,true);
              },false);

              for (var i=startSortColumn; i < header[0].childNodes.length; i++) {
                header[0].childNodes[i].addEventListener("click",function() {
                  sortTable(this,scope,$compile,$timeout);
                },false);
              }
             });
            //htmlElement.append(header);
          }

          htmlElement.append(wrapperElement);
        }
      }
    };
  }

  // globals used for sorting, etc.

  var tdata;           // all our data
  var meta;           // metadata on data sizing
  var htmlElement;    // the html element, which we need to change after sorting
  var header;         // the html element for the table header
  var wrapperElement; // the element that wraps the table (but not the header)

  //
  // if the user clicks on a top-level column header, sort the rows, but only after
  // checking  with the user and reverting any changes in progress
  //

  var sortField;      // field to sort by, if any
  var prevSortElem;   // previous header element, for changing sort style
  var sortForward = true;

  function sortTable(spanElem,scope,$compile,$timeout,sortById) {
    //console.log("sortBy: " + spanElem.innerText + ", meta: " + meta);
    // if it's a new field, sort forward by that field
    if (spanElem !== prevSortElem) {
      if (prevSortElem)
        prevSortElem.firstElementChild.classList.remove("icon", "fa-caret-down", "fa-caret-up");

      prevSortElem = spanElem;

      sortForward = true;
      sortField = spanElem.innerText;
      spanElem.firstElementChild.classList.add("icon", "fa-caret-down");
    }

    // if they clicked the same field, reverse the sort direction
    else {
      if (sortForward) {
        spanElem.firstElementChild.classList.remove("fa-caret-down");
        spanElem.firstElementChild.classList.add("fa-caret-up");
      }
      else {
        spanElem.firstElementChild.classList.remove("fa-caret-up");
        spanElem.firstElementChild.classList.add("fa-caret-down");
      }
      sortForward = !sortForward;

    }

    // now sort the data, clear the div, and render the visible region

    meta["outerKey"] = "data";
    if (sortById)
      tdata.sort(compareById);
    else
      tdata.sort(compare);
    wrapperElement.empty();
    var tableHTML = makeHTMLTopLevel();
    var table = angular.element(tableHTML);
    if (table) {
      $compile(table)(scope,function(compiledTab) {
        wrapperElement.append(compiledTab);
        scope.$applyAsync(function() {});
      }); // must compile to link generated HTML and angular
    }
  }

  // compare two rows based on the sort field
  function compare(a,b) {
    return(myCompare(a,b,sortField,meta));
  }

  // compare two rows based on the doc ID
  function compareById(a,b) {
    var direction = (sortForward ? 1 : -1);
    return a.id.localeCompare(b.id)*direction;
  }

  // since we may need to sort subobjects, make our comparison general
  function myCompare(a,b,sortField,meta) {
    var val1,val2;

    if (meta && meta.outerKey) {
      val1 = a[meta.outerKey][sortField];
      val2 = b[meta.outerKey][sortField];
    }
    else {
      val1 = a[sortField];
      val2 = b[sortField];
    }

    var direction = (sortForward ? 1 : -1);

//  console.log("Got sortField: *" + sortField + "*");
//  console.log("Comparing a: " + JSON.stringify(a));
//  console.log("  to b: " + JSON.stringify(b));
//  console.log("  val1: " + JSON.stringify(val1) + " type: " + (typeof val1));
//  console.log("  val2: " + JSON.stringify(val2) + " type: " + (typeof val2));

    // if one is undefined and the other is not, undefined always goes last
    if ((typeof val1 === 'undefined' || val1 == null) && typeof val2 !== 'undefined')
      return 1 * direction;

    if (typeof val1 !== 'undefined' && (typeof val2 === 'undefined' || val2 == null))
      return -1 * direction;

    if ((typeof val1 === 'undefined' || val1 == null) && (typeof val2 === 'undefined' || val2 == null))
      return 0;

    // do they have the same type? then we can compare
    if (typeof val1 === typeof val2) {
      if (_.isNumber(val1))
        return((val1 - val2) * direction);
      if (_.isBoolean(val1))
        return(val1 == val2 ? 0 : (val1 ? direction : 0));
      if (_.isString(val1))
        return (val1.localeCompare(val2) * direction);

      // typeof array and object is the same, need to see which it is
      if (_.isArray(val1)) {
        if (!_.isArray(val2)) // put objects before arrays
          return(-1 * direction);
        else {
          // how to compare arrays? compare each element until a difference
          for (var i=0; i < Math.min(val1.length,val2.length); i++) {
            var res = myCompare(val1,val2,i);
            if (res != 0)
              return(res);
          }
          // if one array was shorter, put it first
          if (i < val2.length)
            return(1 * direction);
          else if (i < val1.length)
            return(-1 * direction);
          else
            return(0); // two were entirely equal
        }
      }
      if (_.isPlainObject(val1)) { // to compare objects, compare the fields
        for (var key in val1) {
          var res = myCompare(val1,val2,key);
          if (res != 0)
            return(res);
        }
        return(0);
      }
      console.log("shouldn't get here: " + JSON.stringify(val1) + "," + JSON.stringify(val2));
      return(0);
    }

    // types of two values are not equal. Order by bool, number, string, object, array
    if (_.isBoolean(val1))
      return(-1 * direction);
    if (_.isNumber(val1))
      return(-1 * direction);
    if (_.isString(val1))
      return(-1 * direction);
    if (_.isPlainObject(val1))
      return(-1 * direction);

    console.log("shouldn't get here2" +
        ": " + JSON.stringify(val1) + "," + JSON.stringify(val2));

    return(0);
  }


  // avoid HTML injection by changing tag markers to HTML

  var lt = /</gi;
  var gt = />/gi;
  var mySanitize = function(str) {
    if (!str) return ('');
    else if (_.isString(str))
      return(str.replace(lt,'&lt;').replace(gt,'&gt;'));
    else
      return(str);
  };

  //
  // get metadata about the table, columns and widths
  // to create the set of columns, by looking at the fields of every object. If the array
  // only has primitives, then we'll output a single column table listing them.
  // If the array is heterogenous, then some rows will be objects, and some rows will
  // be arrays/primitives
  //

  function getMetaData(object) {
    //
    // traverse the data to figure out what fields are present in every row, and
    // how many columns are needed by each field
    //

    var maxWidth = 250;
    var topLevelKeys = {};
    var columnWidths = {};
    var totalWidth = 0;
    var hasNonObject = false;
    var hasOps = false; // are we looking at top keys with ops/sec?
    var unnamedWidth;
    var rowWidths = [];
    var truncated = false; // did we have to leave out
    for (var row=0; row < object.length; row++)
      if (object[row] && object[row].data && object[row].id && object[row].meta && object[row].meta.type === "json") {
        if (object[row].ops)
          hasOps = true;
        //console.log("row: " + row + ": " + JSON.stringify(object[row].data));
        var data = object[row].data;
        // if the data is a sub-array, or primitive type, they will go in an unnamed column,
        // and figure out how much space it needs
        if (_.isArray(data) || _.isString(data) || _.isNumber(data) || _.isBoolean(data)) {
          hasNonObject = true;
          var width = getColumnWidth(data);
          rowWidths[row] = width;
          if (!unnamedWidth || width > unnamedWidth)
            unnamedWidth = width;
        }

        // otherwise it's an object, loop through its keys
        else _.forEach(object[row].data, function (value, key) {
          topLevelKeys[key] = true;
          // see how much space this value requires, remember the max
          var width = getColumnWidth(value);
          rowWidths[row] = width;
          if (!columnWidths[key] || width > columnWidths[key])
            columnWidths[key] = width;
        });
      }

    _.forIn(topLevelKeys, function(value,key) {
      totalWidth += columnWidths[key];
    });

    // if the total width is > the max number of columns, truncate the data to be presented
    if (totalWidth > maxWidth) {
      var truncatedKeys = {};
      var newWidth = 0;
      _.forIn(topLevelKeys, function(value,key) {
        if ((newWidth + columnWidths[key]) < maxWidth) {
          truncatedKeys[key] = true;
          newWidth += columnWidths[key];
        }
      });

      truncated = true;
      totalWidth = newWidth;
      topLevelKeys = truncatedKeys;
    }


    return({topLevelKeys: topLevelKeys, columnWidths: columnWidths, totalWidth: totalWidth,
      hasNonObject: hasNonObject, unnamedWidth: unnamedWidth, rowWidths: rowWidths, truncated: truncated, hasOps: hasOps});
  }

  //
  // make a header for the table that will be fixed
  //

  function createHTMLheader(meta) {
    //
    // We have widths for each column, so we can create the header row
    //
    var columnHeaders = '<div class="data-table-header-row">';
    columnHeaders += '<span class="data-table-header-cell" style="width:' + columnWidthPx*1.25 + 'px">&nbsp;</span>';
    columnHeaders += '<span class="data-table-header-cell" style="width:' + columnWidthPx*2 + 'px">id<span class="caret-subspan"></span></span>';

    // we may need a column for top key ops/ser
    if (meta.hasOps) {
      columnHeaders += '<span class="data-table-header-cell" style="width: ' +
        0.5*columnWidthPx + 'px;">ops/sec</span>';
    }

    // we may need an unnamed column for things that don't have field names
    if (meta.hasNonObject) {
      columnHeaders += '<span class="data-table-header-cell" style="width: ' +
        meta.unnamedWidth*columnWidthPx + 'px;">&nbsp;</span>';
    }

    // header for each column
    Object.keys(meta.topLevelKeys).sort().forEach(function(key,index) {
      columnHeaders += '<span ng-if="dec.options.show_tables" class="data-table-header-cell" style="width: ' +
      meta.columnWidths[key]*columnWidthPx + 'px;"'

      // for column names too big to fit, add a tooltip
      if (mySanitize(key).length > 25*meta.columnWidths[key])
        columnHeaders += ' title="' + mySanitize(key) + '" ';

      columnHeaders += '>' + mySanitize(key) +'<span class="caret-subspan"></span></span>';
    });
    columnHeaders += '</div>';

    return(columnHeaders);
  }

  //
  // top-level: we have an array of objects, each with a 'data' and 'id' field
  //

  // MAGIC NUMBER: how many pixels wide for each column
  var columnWidthPx = 150;

  function makeHTMLTopLevel() {
    var result = '';
    var max_length = 200; // the old doc editor truncated the JSON at 200 chars, so we will also

    // we expect an array of objects, which we turn into an HTML table.

    // if the array is empty, say so
    if (!_.isArray(tdata) || tdata.length == 0)
      return('<p class="error">No Results</p>');


    var topLevelKeys = meta.topLevelKeys;
    var columnWidths = meta.columnWidths;

    //
    // for each object in the array, output all the possible column values
    //

    for (var row=0; row < tdata.length; row++) {
      // handle JSON docs
      if (tdata[row].id && tdata[row].meta && tdata[row].meta.type === "json")  {// they'd all better have these
        var docTooBig = tdata[row].docSize > 1024*1024;
        var docWayTooBig = tdata[row].docSize > 10*1024*1024;
        var docError = tdata[row].error;
        var formName = 'row' + row + 'Form';
        var pristineName = formName + '.$pristine';
        var setPristineName = formName + '.$setPristine';
        var invalidName = formName + '.$invalid';
        result += '<form name="' + formName + '" style="width: ' + (meta.totalWidth + meta.unnamedWidth + 3.25)*columnWidthPx + 'px" ' +
        ' ng-submit="dec.updateDoc(' + row +',' + formName + ')">' +
        '<fieldset class="doc-editor-fieldset" ng-disabled="!rbac.cluster.bucket[dec.options.selected_bucket].data.docs.upsert">' +
        '<div class="doc-editor-row" ' +
        'ng-if="!dec.options.current_result[' + row + '].deleted">'; // new row for each object

        result += '<span class="doc-editor-cell" style="width:' + columnWidthPx*1.25 + 'px"> ' +

        '<a class="btn square-button" ' +
        'ng-disabled="' + invalidName + ' || ' + docError + '" ' +
        'ng-click="dec.editDoc(' + row +',!rbac.cluster.bucket[dec.options.selected_bucket].data.docs.upsert)" ' +
        'title="Edit document as JSON"><span class="icon fa-pencil"></span></a>' +

        '<a class="btn square-button" ' +
        'ng-disabled="' + invalidName + ' || ' + docError + ' || !rbac.cluster.bucket[dec.options.selected_bucket].data.docs.upsert" ' +
        'ng-click="dec.copyDoc(' + row +',' + formName +')" ' +
        'title="Make a copy of this document"><span class="icon fa-copy"></span></a>' +

        '<a class="btn square-button" ' +
        'ng-disabled="!rbac.cluster.bucket[dec.options.selected_bucket].data.docs.upsert' + ' || ' + docError + '" ' +
        'ng-click="dec.deleteDoc(' + row +')" ' +
        'title="Delete this document"><span class="icon fa-trash"></span></a>' +

        '<a class="btn square-button" ' +
        'ng-disabled="' + pristineName + ' || '+ invalidName + ' || ' + docError + '" ' +
        'ng-click="dec.updateDoc(' + row +',' + formName + ')" ' +
        'title="Save changes to document"><span class="icon fa-save"></span></a>' +

        '</span>';

        // put the meta().id in the next column
        result += '<span class="doc-editor-cell" style="width:' + columnWidthPx*2 + 'px">';

        if (!docWayTooBig)
          result += '<a ng-click="dec.editDoc(' + row +',!rbac.cluster.bucket[dec.options.selected_bucket].data.docs.upsert)">';
        else
          result += '<a>';

        result += mySanitize(tdata[row].id);
        if (docWayTooBig)
          result += ' <span class="icon fa-exclamation-triangle" ' +
          'uib-tooltip-html="\'Document is too large for editing in the browser: ' + Math.round(tdata[row].docSize*10/(1024*1024))/10 + 'MB.\'"' +
          'tooltip-placement="auto right" tooltip-append-to-body="true" tooltip-trigger="\'mouseenter\'">';
        else if (docTooBig)
          result += ' <span class="icon fa-exclamation-triangle" ' +
          'uib-tooltip-html="\'Document is ' + Math.round(tdata[row].docSize*10/(1024*1024))/10 + 'MB, editing will be slow.\'"' +
          'tooltip-placement="auto right" tooltip-append-to-body="true" tooltip-trigger="\'mouseenter\'">';
        else if (tdata[row].rawJSONError)
          result += ' <span class="icon fa-exclamation-triangle" ng-if="dec.options.show_tables"' +
                       'uib-tooltip-html="\'Error checking document for numbers too long to edit. Tabular editing not permitted. ' +
                       tdata[row].rawJsonError + '\'"' +
                       'tooltip-placement="auto right" tooltip-append-to-body="true" tooltip-trigger="\'mouseenter\'">';
        else if (tdata[row].rawJSON)
          result += ' <span class="icon fa-exclamation-triangle" ng-if="dec.options.show_tables"' +
                       'uib-tooltip-html="\'Document contains numbers too large for tabular editing, click doc id to edit as JSON .\'"' +
                       'tooltip-placement="auto right" tooltip-append-to-body="true" tooltip-trigger="\'mouseenter\'">';
        result += '</a></span>';

        // if we are showing top keys, add the ops per second
        if (meta.hasOps) {
          if (tdata[row].ops && _.isNumber(tdata[row].ops))
            tdata[row].ops = Math.round(tdata[row].ops*1000)/1000;

          result += '<span class="doc-editor-cell" style="width:' + 0.5*columnWidthPx + 'px">' + tdata[row].ops + '</span>';
        }


        // if we have unnamed items like arrays or primitives, they go in the next column
        if (meta.hasNonObject) {
          result += '<span ng-if="dec.options.show_tables" class="doc-editor-cell" style="width:' +
          meta.unnamedWidth*columnWidthPx + 'px">';

          // if this row is a subarray or primitive, put it here
          var data = tdata[row].data;
          if (_.isArray(data) || _.isString(data) || _.isNumber(data) || _.isBoolean(data)) {
            var childSize = {width: 1};
            result += makeHTMLtable(data,'[' + row + '].data',childSize);
          }

          result += '</span>';
        }

        // now the field values, if we are showing tables, but only if we have a non-null document

        if (tdata[row].data || tdata[row].data === 0) {
          Object.keys(meta.topLevelKeys).sort().forEach(function(key,index) {
            var item = tdata[row].data[key];
            var childSize = {width: 1};
            var disabled = !!tdata[row].rawJSON || docTooBig || docError;
            var childHTML = (item || item === 0 || item === "" || item === false) ?
                makeHTMLtable(item,'[' + row + '].data[\''+ key.replace(/'/g,"\\'") + '\']', childSize, disabled) : '&nbsp;';
                result += '<span ng-if="dec.options.show_tables" class="doc-editor-cell" style="width: ' +
                columnWidths[key]*columnWidthPx  + 'px;">' + childHTML + '</span>';
          });

          // otherwise, a truncated version of the JSON

          var json = tdata[row].rawJSON || JSON.stringify(tdata[row].data);
          if (json.length > max_length)
            json = json.substring(0,max_length) + '...';
          result += '<span ng-if="!dec.options.show_tables" class="doc-editor-cell" style="width: ' + 5*columnWidthPx  + 'px;">'
          + mySanitize(json) + '</span>';
        }

        // for a null document, output a message saying so

        else {
          result += '<span class="doc-editor-cell" style="width: ' + 5*columnWidthPx  + 'px;">Empty Document</span>';
        }

        // for some reason I couldn't get the form from $scope, so the following acts as a sentinel that I can search for
        // to see if anything changed in any form of the editor
        result +=  '<span id="somethingChangedInTheEditor" ng-if="!' + pristineName + '"></span>';

        result += '</div></fieldset></form>'; // end of the row for the top level object
      }

      // what to show for BINARY documents? They're not editable
      else if (tdata[row].meta && tdata[row].meta.type === "base64") {
        result += '<form name="' + formName + '">' + '<div class="doc-editor-row" ' +
          'ng-if="!dec.options.current_result[' + row + '].deleted">'; // new row for each object

        // span where the buttons would go, all disabled except include delete
        result += '<span class="doc-editor-cell" style="width:' + columnWidthPx*1.25 + 'px"> ' +
        '<a class="btn square-button" ng-disabled="true"><span class="icon fa-edit"></span></a>' +

        '<a class="btn square-button" ng-disabled="true"><span class="icon fa-copy"></span></a>' +

        '<a class="btn square-button" ng-click="dec.deleteDoc(' + row +')" ' +
        'title="Delete this document"><span class="icon fa-trash"></span></a>' +

        '<a class="btn square-button" ng-disabled="true"><span class="icon fa-save"></span></a>' +

        '</span>';

        // and the id and metadata
        result += '<span class="doc-editor-cell" style="width: ' + 2*columnWidthPx  +
          'px;"><span '
        if (tdata[row].meta || tdata[row].xattrs)
          result += 'class="cursor-pointer blue-1" uib-tooltip-html="{{getTooltip(' + row + ')}}" ' +
          'tooltip-placement="auto bottom" tooltip-is-open="showTT'+row+' && !dec.hideAllTooltips" tooltip-entooltip-append-to-body="true" ' +
          'tooltip-trigger="\'none\'" data-ng-click="showTT'+row+' = !showTT'+row+ '"';
        result += '>' + mySanitize(tdata[row].id) + '</span></span>';

        var binary = tdata[row].base64 ? tdata[row].base64.substring(0,150) : " base64 not available";
        if (tdata[row].base64 && tdata[row].base64.length > 150)
          binary += "...";

        //console.log("Got row: " + JSON.stringify(data[row]));

        result += '<span class="doc-editor-cell" style="width: 100%;">Binary Document, ' + binary + '</span>';

        result += '</div></form>'; // end of the row for the top level object
      }
    }

    //console.log("After header, loop: " + result);


    // done with array, close the table
    //result = '<div class="data-table" style="width: ' + (totalWidth+3)*(columnWidthPx+10) + 'px; overflow: auto">'
    //+ result + '</div>';

    return(result);
  }

  function getTooltip(row) {
    var meta = {};
    if (tdata[row].data)
      meta.doc_size = JSON.stringify(tdata[row].data).length;
    meta.meta = tdata[row].meta;
    meta.xattrs = tdata[row].xattrs;
    return("'" + JSON.stringify(meta,null,2).replace(/\n/g,'<br>').replace(/ /g,'&nbsp;') + "'");
  }


  // The data we are showing can contain objects nested inside objects inside objects: turtles
  // all the way down. When making a table, a piece of data could be 1 column wide, or 100, or 1000.
  // This function recursively traverses a piece of data to figure out how many columns wide it
  // will need.

  function getColumnWidth(item) {
    // arrays are complex - an array of primitives is a vertical list of values, but a mixed
    // array of objects and primitives is shown as a table, with an unnamed column for the
    // primitives
    if (_.isArray(item)) { // arrays will list vertically, find max width of any element
      //console.log("Getting width for array: " + JSON.stringify(item));
      var namedWidth = 1;
      var unnamedWidth = 0;
      for (var i=0; i < item.length; i++) {
        var elementWidth = getColumnWidth(item[i]);

        // is the item named (object) or unnamed (prim or array)
        if (_.isArray(item[i]) || _.isString(item[i]) || _.isNumber(item[i]) || _.isBoolean(item[i])) {
          if (elementWidth > unnamedWidth)
            unnamedWidth = elementWidth;
        }
        else {
          if (elementWidth > namedWidth)
            namedWidth = elementWidth;
        }
      }

      //console.log("   got array width: " + namedWidth + unnamedWidth);
      return(namedWidth + unnamedWidth);
    }

    else if (_.isPlainObject(item)) { // for objects we need to sum up columns for each field
      //console.log("Getting width for object: " + JSON.stringify(item));
      var totalWidth = 0;
      _.forIn(item, function (value, key) { // for each field
        totalWidth += getColumnWidth(value);
      });

      if (totalWidth == 0)
        totalWidth = 1;

      //console.log("   got object width: " + totalWidth);
      return(totalWidth);
    }

    else // all primitive types just get 1 column wide.
      return 1;
  }


  //recursion in Angular is really inefficient, so we will use a javascript
  //routine to convert the object to an HTML representation. It's not true to
  //the spririt of Angular, but it was taking 10 seconds or more to render
  //a table with tens of thousands of cells

  var max_array_len = 100; // don't show arrays longer than this in overly large documents

  function makeHTMLtable(object,prefix,totalSize,disabled) {
    var result = '';

    // we might get a simple value to render, or an object, or an array of objects.
    // we need to return the total width of what we render, so the caller, knows
    // how much space to allow
    //
    // if we get an array of objects, it is a sub-table which we turn into an CSS
    // table. the first step is
    // to create the set of columns, by looking at the fields of every object. If the array
    // only has primitives, then we'll output a single column table listing them.
    // If the array is heterogenous, then some rows will be objects, and some rows will
    // be arrays/primitives

    if (_.isArray(object)) {
      // if the array is empty, say so
      if (object.length == 0)
        return('<div class="ajtd-key">[ ]</div>');

      // find the columns
      var arrayObjCount = 0;
      var arrayArrayCount = 0;
      var arrayPrimCount = 0;
      var itemsKeysToObject = {};
      _.forEach(object, function (item, index) {
        if (_.isPlainObject(item)) {
          arrayObjCount++;
          _.forIn(item, function (value, key) {
            itemsKeysToObject[key] = true;
          });
        }
        else if (_.isArray(item))
          arrayArrayCount++;
        else
          arrayPrimCount++;
      });

      //
      // special case: an array of primitive types or arrays. output them as a
      // single width vertical list
      //

      if (arrayObjCount == 0)  {
        result += '<div>';
        var childSize = {width: 1};

        _.forEach(object, function (item,index) {
          result += makeHTMLtable(item, prefix + '[' + index + ']', childSize, disabled) + '<br>';
          // remember the widest element
          if (childSize.width > totalSize.width)
            totalSize.width = childSize.width;
        });
        result += '</div>';

        return(result);
      }

      //
      // another special case: whenever the user does "select * from <bucket>", they
      // get an array of objects with only one field, whose key is the bucket name and
      // whose value is an abject. In that case we get a really ugly table, with a subtable
      // for each row. To work around this, in the case where we found only one column, "peek"
      // inside to allow access to those inner fields
      //

      var innerKeys;
      var fields = Object.keys(itemsKeysToObject);
      if (fields.length == 1) {
        var onlyField = fields[0];

        // loop through the array
        _.forEach(object, function (item, index) {
          // if the item is an object
          if (_.isPlainObject(item[onlyField])) {
            _.forIn(item[onlyField], function (value, key) {
              if (!innerKeys)
                innerKeys = {};
              innerKeys[key] = true;
            });
          }
        });
      }

      // otherwise, we have an array of objects and/or primitives & arrays.
      // Make a table whose columns are the union of all fields in all the objects. If we
      // have a non-object, output it as a full-width cell.
      var keys = (innerKeys ? innerKeys : itemsKeysToObject);

      // need to keep track of the widths of each column so the headers can know what size to be
      var columnWidths = {};
      var unnamedWidth; // width for non-field values, which don't have a name
      _.forIn(keys, function (value, key) { // set default width for each column
        columnWidths[key] = 1;
      });

      totalSize.width = 0;
      // get the max width for each column of each row
      _.forEach(object, function (item, index) {
        // limit these arrays to 100 items
        if (index > max_array_len)
          return(false);
        // if the row is an array or primitive, compute the width for the unnamed column
        if (_.isArray(item) || _.isString(item) || _.isNumber(item) || _.isBoolean(item)) {
          var width = getColumnWidth(item);
          if (!unnamedWidth || width > unnamedWidth)
            unnamedWidth = width;
        }

        else if (_.isPlainObject(item)) {
          // if we are using innerKeys, get to the inner object
          if (innerKeys)
            item = item[onlyField];

          // if it's an empty object, just say so
          if (_.keys(item).length > 0)
            _.forIn(keys, function(b,key) {
              var value = item ? item[key] : null;

              if (value && getColumnWidth(value) > columnWidths[key])
                columnWidths[key] = getColumnWidth(value);
            });

        }
      });

      // now we know the widths of each key, compute total width
      for (var key in columnWidths) {
        //console.log("For key: " + key + " got width: " + columnWidths[key]);
        totalSize.width += columnWidths[key];
      }
      if (unnamedWidth)
        totalSize.width += unnamedWidth;

      //console.log("Got totalWidth: " + totalSize.width);

      // for each object in the array, output all the column (and unnamed) values
      _.forEach(object, function (item, index) {
        // limit these arrays to 100 items
        if (index > max_array_len) {
          result += '<div class="doc-editor-row">Array length ' + object.length +
            ' truncated to ' + max_array_len + ' rows, use JSON editing to see entire array</div>';
          return false;
        }


        result += '<div class="doc-editor-row">'; // new div for each row

        // if there exist unnamed objects in the array, output them in the first column
        if (unnamedWidth) {
          result += '<span class="doc-editor-cell" style="width: ' + unnamedWidth*columnWidthPx + 'px">';

          // is this row an array or primitive?
          if (_.isArray(item) || _.isString(item) || _.isNumber(item) || _.isBoolean(item)) {
            var childSize = {width: 1};
            result += makeHTMLtable(item,prefix + "[" + index + "]", childSize, disabled);
          }

          result += '</span>';

        }

        if (_.isPlainObject(item)) {
          // if we are using innerKeys, get to the inner object
          if (innerKeys)
            item = item[onlyField];

          // if it's an empty object, just say so
          if (_.keys(item).length == 0)
            result += '<span class="doc-editor-cell" style="width: ' + columnWidthPx + 'px">empty object</span>';

          else _.forIn(keys, function(b,key) {
            var value = item ? item[key] : null;

            //console.log("  key: " + key + ", value: " + value);

            // for objects and arrays, make a recursive call
            if (_.isArray(value) || _.isPlainObject(value)) {
              var childSize = {width: 1};
              var childHTML = makeHTMLtable(value,prefix + '[' + index + ']' +
                  (innerKeys ? ('.' + onlyField) : '') +
                  '[\''+ key + '\']',childSize, disabled);
              result += '<span class="doc-editor-row" style="width: '+ childSize.width*columnWidthPx + 'px;">' + childHTML + '</span>';
            }

            // primitive values also use an input form generated recursively
            else {
              var childSize = {width: 1};
              var childHTML = makeHTMLtable(value,prefix + '[' + index + ']' +
                  (innerKeys ? ('.' + onlyField) : '') +
                  '[\''+ key + '\']',childSize, disabled);
              result += '<span class="doc-editor-row" style="width: '+ childSize.width*columnWidthPx + 'px;">' + childHTML + '</span>';
            }

          });

        }

        // if the item wasn't an object, add filler cells for the empty fields
        else _.forIn(keys, function(b,key) {
          result += '<span class="doc-editor-cell empty" style="width: ' + columnWidthPx + 'px"></span>';
        });

        result += '</div>'; // end the row
      });

      // done with the array of values, now know how big each column header should be
      var columnHeaders = '';
      var arrayWidth = 0;
      // if we have unnamed items, leave a blank-headered column for them
      if (unnamedWidth) {
        columnHeaders += '<span class="data-table-header-cell" style="width: ' +
          unnamedWidth*columnWidthPx + 'px;">&nbsp;</span>';
        arrayWidth += unnamedWidth * columnWidthPx;
      }

      // now column headers for fields we saw in the array
      _.forIn(keys, function(value,key) {
        columnHeaders += '<span class="data-table-header-cell" style="width: ' +
          columnWidths[key]*columnWidthPx + 'px;">' + mySanitize(key) +'</span>';
        arrayWidth +=  columnWidths[key]*columnWidthPx;
      });

      // finish the table
      result = '<div class="data-table-editor-header-row" style="width: ' + arrayWidth + 'px">' +
      columnHeaders + '</div>' + result;
    }

    //
    // instead of an array, we might get an object, create a table and iterate over the fields
    // what is the right way to do this? An object could be shown as a horizontal table, with
    // one column per field. It also could be shown as a vertical table, with one row per field.
    // For the purposes of an editor, we will make the sub-object horizontal, to offset the
    // field names in a title bar
    //

    else if (_.isPlainObject(object)) {

      var columnWidths = {};

      var dataRow = '<div class="doc-editor-row">';

      // figure out the widths of each column
      _.forIn(object, function(value,key) {
        var childSize = {width: 1};
        var childHTML = makeHTMLtable(value,prefix + "['" + key.replace(/'/g,"\\'") + "']",childSize, disabled);
        if (!columnWidths[key] || childSize.width > columnWidths[key])
          columnWidths[key] = childSize.width;
        dataRow += '<span class="doc-editor-cell" style="width: ' + columnWidths[key]*columnWidthPx + 'px">' + childHTML + '</span>';
      });
      dataRow += '</div>';

      // row of field names
      var totalWidth = 0;
      var columnHeaders = '<div class="data-table-editor-header-row">';
      _.forIn(object, function(value,key) {
        columnHeaders += '<span class="doc-editor-cell" style="width: ' +
        columnWidths[key]*columnWidthPx + 'px;">' + mySanitize(key) +'</span>';
        totalWidth += columnWidths[key];
      });
      columnHeaders += '</div>';

      totalSize.width = totalWidth;
      result += columnHeaders + dataRow;
    }

    //it's also possible we were passed a primitive value, in which case just put it in a div

    else {
      var model = ' ng-model="results' + prefix + '" ';
      var inputStyle = ' style="width: ' + (columnWidthPx-10) + 'px; margin-left: 0px"';
      var no_edit = disabled ? ' ng-disabled="true" ' : '';

      if (_.isNumber(object))
        result += '<input type="number" step="any" ' + model + inputStyle + no_edit + '>';
      else if (_.isBoolean(object))
        result += '<select ' + model + inputStyle + no_edit +
        ' ng-options="opt.v as opt.n for opt in [{n: \'false\', v: false}, {n:\'true\', v: true}]"></select>';

      // can't edit incredibly long strings without the browser barfing
      else if (object && object.length > 1024*512)
        result += '<div class="text-center"><span class="icon fa-exclamation-triangle" ' +
        'uib-tooltip-html="\'Field value too large to edit in spreadsheet mode. Try editing as JSON.\'"' +
        'tooltip-placement="auto right" tooltip-append-to-body="true" tooltip-trigger="\'mouseenter\'">' +
        '</span></div>';
      else
        result += '<textarea ' + model + inputStyle + no_edit + '></textarea>';
    }

    return(result);
  };

})();
