(function() {

  //
  // the qwJsonCsvService contains utility functions for converting
  // JSON documents to CSV
  //

  angular.module('qwQuery').factory('qwJsonCsvService', getQwJsonCsvService);

  getQwJsonCsvService.$inject = [];

  function getQwJsonCsvService() {

    var qwJsonCsvService = {};

    //
    // convert an array of documents to CSV. We need to run through the documents, and find
    // the top level fields. If the fields have sub-documents, we can have them as fields whose
    // name are paths. The only thing we can't handle is arrays, which are just output as strings.
    //

    qwJsonCsvService.convertDocArrayToCSV = convertDocArrayToCSV;
    qwJsonCsvService.convertDocArrayToTSV = convertDocArrayToTSV;

    function convertDocArrayToCSV(docArray) {
      var dataArray = convertDocArrayToDataArray(docArray);
      var result = "";
      for (var i=0; i < dataArray.length; i++)
        result += dataArray[i].join(',') + '\n';

      return(result);
    }

    function convertDocArrayToTSV(docArray) {
      var dataArray = convertDocArrayToDataArray(docArray);
      var result = "";
      for (var i=0; i < dataArray.length; i++)
        result += dataArray[i].join('\t') + '\n';

      return(result);
    }

    //
    // convert documents to an array of values, that we can delimit different ways
    //

    function convertDocArrayToDataArray(docArray) {
      var data = [];
      var fieldInfo = {};
      //console.log("Converting result to CSV: " + _.isArray(docArray) + ", " + docArray.length);

      // we need an array
      if (!_.isArray(docArray) || docArray.length == 0)
        return(data);

      // figure out what fields we have available, by looking at each doc
      for (var i=0;i<docArray.length;i++) {
        getFields(docArray[i],fieldInfo);
      }

      // if there is only one key in fieldInfo, and it's an object type, use the inner object instead
      var topLevelFields = Object.keys(fieldInfo);
      var innerKey;
      var firstField = (topLevelFields.length > 0 ? topLevelFields[0] : "");
      if (topLevelFields.length == 1 && fieldInfo[firstField].obj && !fieldInfo[firstField].nonobj) {
        innerKey = firstField;
        fieldInfo = fieldInfo[innerKey].obj;
      }

      //console.log("Got fieldInfo: " + JSON.stringify(fieldInfo,null,3));

      //
      // get the column names as an array, delimiter can come later
      //

      var nameArray = [];
      getFieldNames(fieldInfo,nameArray,"");
      //console.log("name array: " + JSON.stringify(nameArray));

      //
      // now get each document as an array of values, to be output as a row of data
      //

      data.push(nameArray);
      for (var i=0;i<docArray.length;i++) {
        var valArray = [];
        var doc = docArray[i];
        if (innerKey)
          doc = doc[innerKey];
        convertDocToArray(doc,fieldInfo,valArray);
        data.push(valArray);
      }

      return(data);
    }


    //
    // find all the fields in a document
    //

    function getFields(doc,fieldInfo) {
      for (var key in doc) {
        var value = doc[key];
         if (!fieldInfo[key])
          fieldInfo[key] = {nonobj:false, obj: null}; // does the field include object values? non-object?

        if (_.isNumber(value) || _.isString(value) || _.isBoolean(value) || _.isArray(value))
          fieldInfo[key].nonobj = true;

        else if (_.isPlainObject(value)) {
          fieldInfo[key].obj = {};
          getFields(value,fieldInfo[key].obj);
        }
      }
    }


    //
    // given the field information collected by the above function,
    // turn it into an array of field names, including paths for subfields,
    // that will be used for the first line of the CSV
    //

    function getFieldNames(fieldInfo, nameArray, prefix) {
      Object.keys(fieldInfo).sort().forEach(function(fieldName,index) {
        var field = fieldInfo[fieldName];

        // if the field can be a primitive or array, we want the basic name
        if (field.nonobj)
          nameArray.push(prefix + fieldName);

        // if the field can be an object, we need to find paths to all subobjects
        if (field.obj)
          getFieldNames(field.obj,nameArray, prefix + fieldName + ".");
      });
    }


    //
    // given the field info, turn a document into an array of values, one for each field
    //

    function convertDocToArray(doc,fieldInfo,valArray) {

      // for each field...
      Object.keys(fieldInfo).sort().forEach(function(fieldName,index) {
        var field = fieldInfo[fieldName]; // name
        var value = doc ? doc[fieldName] : null;       // value

        // if the value can be a primitive or array, we output a value for it
        if (field.nonobj) {
           if (_.isNumber(value) || _.isBoolean(value))
            valArray.push(value);

          else if (_.isString(value) || _.isArray(value))
            valArray.push(JSON.stringify(value));

          else // no value
            valArray.push(null);
        }

        // if the value can be an object, output all subdoc fields
        if (field.obj)
          convertDocToArray(value,fieldInfo[fieldName].obj,valArray);

      });

    }


    return qwJsonCsvService;
  }



})();
