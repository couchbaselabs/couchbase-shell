(function () {
  "use strict";

  angular
    .module('mnD3Service', [])
    .factory('mnD3Service', mnD3ServiceFactory);

  function mnD3ServiceFactory() {
    class mnD3 {
      constructor(options, rootElement) {
        this.opt = options;
        this.cht = this.opt.chart;
        this.rootEl = d3.select(rootElement);
        this.throttledResize = _.throttle(this.resize.bind(this), 30);
        var elmRect = this.getElementRect();
        this.cvsRect = this.getCanvasRect(elmRect);
        this.colors = this.cht.color || d3.schemeCategory10;

        //main container
        this.svg =
          this.rootEl.append("svg").attr("width", "100%").attr("height", this.cht.height)
          .append("g").attr("transform", this.getTransform(this.cht.margin.left,
                                                           this.cht.margin.top));
      }
      init() {
        this.inititalized = true;
        this.svg.html("");

        this.linesWrap = this.svg.append("g")
          .attr("clip-path", "url(#clip)");

        // Initialise a X axis:
        this.xScale = d3.scaleTime().range([0, this.cvsRect.width]);
        this.xAxis = d3.axisBottom().scale(this.xScale).tickFormat(this.cht.xAxis.tickFormat);
        this.svg.append("g").attr("transform", this.getTransform(0, this.cvsRect.height))
          .attr("class", "xAxis");

        // Initialise a Y axis and lines
        this.yScale = [];
        this.yAxis = [];
        this.yLines = [];
        this.createYAxis(0, "axisLeft", this.getTransform(0, 0));
        if (this.cht.yAxis[1]) {
          this.createYAxis(1, "axisRight", this.getTransform(this.cvsRect.width, 0));
        }
      }
      destroy() {
        this.svg.remove();
      }
      getCanvasRect(elmRect) {
        return {
          width: elmRect.width - this.cht.margin.left - this.cht.margin.right,
          height: this.cht.height - this.cht.margin.bottom - this.cht.margin.top
        };
      }
      updateYAxis(i) {
        var domain = this.data.filter(function (line) {
          return (line.yAxis == i) && this.filterDisabled(line);
        }.bind(this));

        var yDomain = this.cht.yAxis[i].domain(domain);

        this.yScale[i].domain(yDomain);
        if (!this.cht.showTicks && domain.length) {
          this.yAxis[i].tickValues(yDomain);//show min/max only
        }

        this.svg.selectAll(".yAxis" + i).call(this.yAxis[i]).transition().duration(0);
      }
      createYAxis(i, axis, coordinates) {
        this.yScale[i] = d3.scaleLinear().range([this.cvsRect.height, 0]).nice();
        this.yAxis[i] = d3[axis]().scale(this.yScale[i]).tickFormat(this.cht.yAxis[i].tickFormat);

        this.yLines[i] = d3.line()
          .defined(function (d) {
            return !isNaN(d[1]);
          })
          .x(function (d) {
            return this.xScale(d[0]);
          }.bind(this))
          .y(function (d) {
            return this.yScale[i](d[1]);
          }.bind(this));

        if (!this.cht.hideTicks) {
          this.svg.append("g").attr("transform", coordinates).attr("class", "yAxis yAxis" + i);
        }
      }
      getElementRect() {
        return this.rootEl.node().getBoundingClientRect();
      }
      getTransform(x, y) {
        return "translate(" + x + ", " + y + ")";
      }
      filterDisabled(line) {
        return !!line.values.length && !line.disabled;
      }
      redrawXAxis() {
        return this.svg.selectAll(".xAxis").transition().duration(0).call(this.xAxis);
      }
      getLineColor(d, i) {
        var color = this.colors
            .slice(0, this.data.length)
            .filter(function (_, i) {
              return !!this.data[i].values.length && !this.data[i].disabled;
            }.bind(this));
        return color[i % color.length];
      }
      drawLine(path) {
        path.attr("d", function (d) {
          if (!d.disabled) {
            return this.yLines[d.yAxis](d.values);
          } else {
            return undefined;
          }
        }.bind(this))
          .call(this.drawLinePath.bind(this));
      }
      drawLinePath(pipe) {
        pipe.style("stroke", this.getLineColor.bind(this))
          .style("fill", "none")
          .style("stroke-width", 1);
      }
      drawCirclePath(pipe) {
        pipe.style("fill", this.getLineColor.bind(this));
      }
      updateXAxis(xDomain) {
        if (!xDomain.length) {
          return;
        }
        this.xScale.domain(xDomain);
        this.redrawXAxis();
      }
      updateLine() {
        this.linesWrap.selectAll('.lines')
          .data(this.data.filter(this.filterDisabled.bind(this)))
          .join(function (enter) {
            enter
              .append('path').attr('class', 'lines')
              .call(this.drawLine.bind(this));
          }.bind(this), function (update) {
            update
              .transition().duration(0)
              .call(this.drawLine.bind(this));
          }.bind(this));
      }
      updateData(data) {
        if (!data) {
          return;
        }

        this.xAxisData = data
          .reduce((max, item) =>
                  item.values.length > max.length ? item.values : max
                  , []);
        this.data = data;

        if (this.xAxisData && this.xAxisData.length) {
          if (!this.inititalized) {
            this.init();
          }
        } else {
          this.showEmptyContent();
          return;
        }

        this.updateXAxis(this.getXaxisDomain(this.xAxisData));

        this.updateYAxis(0);

        if (this.cht.yAxis[1]) {
          this.updateYAxis(1);
        }

        this.updateLine();
        return true;
      }
      showEmptyContent() {
        this.inititalized = false;
        this.svg.html("<text class='charts-nodata'>"+this.cht.noData+"</text>");
      }
      getXaxisDomain(data) {
        return [data[0][0], data[data.length-1][0]];
      }
      toggleLine(i) {
        var maybeLast = this.data.filter(this.filterDisabled.bind(this));
        if ((maybeLast.length == 1) && (this.data.indexOf(maybeLast[0]) == i)) {
          return;
        }
        this.data[i].disabled = !this.data[i].disabled;
        this.updateData(this.data);
        return true;
      }
      resize() {
        this.cvsRect = this.getCanvasRect(this.getElementRect());
        this.xScale.range([0, this.cvsRect.width]);
        this.redrawXAxis();
        this.xAxis.ticks(Math.max(this.cvsRect.width/100, 2));
        if (this.cht.yAxis[1] && !this.cht.hideTicks) {
          this.svg.select(".yAxis1").attr("transform", this.getTransform(this.cvsRect.width, 0));
        }
      }
    }

    class mnD3Focus extends mnD3 {
      constructor(options, rootElement, chart) {
        super(options, rootElement[0]);
        this.chart = chart;
      }
      init() {
        super.init();

        this.chart.clip = this.svg.append("defs").append("svg:clipPath")
          .attr("id", "clip")
          .append("svg:rect")
          .attr("width", this.chart.cvsRect.width)
          .attr("height", this.chart.cvsRect.height)
          .attr("x", 0)
          .attr("y", 0);

        this.chart.linesWrap.attr("clip-path", "url(#clip)");

        this.bisect = d3.bisector(function (d) { return d[0]; }).left;

        this.brush = d3.brushX()
          .extent([[0, 0], [this.cvsRect.width,
                            this.cht.height]])
          .on("brush end", this.brushed.bind(this));

        this.brushEl = this.svg.append("g")
          .attr("class", "charts-brush");

        this.svg.attr("class", "focus-chart");

        this.brushEl
          .call(this.brush)
          .call(this.brush.move, null);

        this.onInit && this.onInit();
      }
      getDomain() {
        var s = d3.brushSelection(this.brushEl.node());
        return s ? s.map(this.xScale.invert, this.xScale) : this.getXaxisDomain(this.xAxisData);
      }
      brushed() {
        if (!this.data) {
          return;
        }
        var domain = this.getDomain();

        this.chart.updateXAxis(domain);
        this.chart.updateLine();
        this.chart.drawTooltip();
      }
      updateData(data) {
        if (!super.updateData(data)) {
          return;
        }
        this.brushed();
      }
      resize() {
        var s = this.getDomain();

        super.resize();

        this.chart.clip
          .attr("width", this.chart.cvsRect.width)
          .attr("height", this.chart.cvsRect.height);

        this.brush.extent([[0, 0], [this.cvsRect.width, this.cht.height]]);
        this.brushEl.call(this.brush);

        if (d3.brushSelection(this.brushEl.node())) { //proportional resize of selection
          var i1 = this.bisect(this.xAxisData, s[0]);
          var i2 = this.bisect(this.xAxisData, s[1]);

          this.brush.move(this.brushEl, [
            this.xScale(this.xAxisData[i1][0]),
            this.xScale(this.xAxisData[i2][0])
          ]);

          this.brushed();
        }
      }
    }

    class mnD3Tooltip extends mnD3 {
      constructor(options, rootElement, onInit) {
        super(options, rootElement[0]);
        this.onInit = onInit;
      }
      init() {
        super.init();

        this.bisect = d3.bisector(function (d) { return d[0]; }).left;

        //Tooltip
        this.tip = d3.select("body").append("div").attr('class', 'mnd3-tooltip');
        this.tipLineWrap = this.svg.append("g").attr("class", "tip-line-wrap");
        this.tipLineWrap.append("path").attr("class", "tip-line").style("opacity", 0);
        this.tipBox = this.svg.append('rect')
          .attr("height", this.cvsRect.height)
          .attr("width", this.cvsRect.width).attr('opacity', 0);

        this.drawTooltipThrottle = _.throttle(this.drawTooltip.bind(this), 10, {leading: true});

        angular.element(this.tipBox.node()).on('mousemove', this.setMouseMoveEvent.bind(this));
        angular.element(this.tipBox.node()).on('mousemove', this.drawTooltipThrottle);
        angular.element(this.tipBox.node()).on('mouseout', this.hideTooltip.bind(this));

        this.onInit && this.onInit();
      }
      destroy() {
        angular.element(this.tipBox.node()).off('mousemove', this.setMouseMoveEvent.bind(this));
        angular.element(this.tipBox.node()).off('mousemove', this.drawTooltipThrottle);
        angular.element(this.tipBox.node()).off('mouseout', this.hideTooltip.bind(this));
        this.getLegends().nodes().forEach(function (node, i) {
          angular.element(node).off('click', this.clickCB[i]);
        }.bind(this));
        this.tip.remove();
        this.svg.remove();
        this.legendsWrap.remove();
      }
      showEmptyContent() {
        super.showEmptyContent();
        this.tip && this.tip.remove();
        this.legendsWrap && this.legendsWrap.remove();
      }
      updateData(data) {
        if (!super.updateData(data)) {
          return;
        }
        this.drawTooltip();
      }

      toggleLine(i) {
        if (!super.toggleLine(i)) {
          return;
        }
        d3.select(this.getLegends.bind(this)().nodes()[i]).classed('disabled', this.data[i].disabled);
      }
      resize() {
        super.resize();
        this.tipBox && this.tipBox.attr("height", this.cvsRect.height).attr("width", this.cvsRect.width);
      }
    }

    mnD3Tooltip.prototype.updateLabelRow = updateLabelRow;
    mnD3Tooltip.prototype.updateCirclePosition = updateCirclePosition;
    mnD3Tooltip.prototype.drawTooltip = drawTooltip;
    mnD3Tooltip.prototype.drawCircle = drawCircle;
    mnD3Tooltip.prototype.setMouseMoveEvent = setMouseMoveEvent;
    mnD3Tooltip.prototype.hideTooltip = hideTooltip;
    mnD3Tooltip.prototype.disableTooltip = disableTooltip;
    mnD3Tooltip.prototype.drawLegends = drawLegends;
    mnD3Tooltip.prototype.getLegends = getLegends;


    function getLegends() {
      return this.legendsWrap.selectAll('.legends');
    }

    function drawCircle(path) {
      path
        .call(this.drawCirclePath.bind(this))
        .call(this.updateCirclePosition.bind(this), this.selectedValueIndex);
    }

    function drawTooltip() {
      if (!this.mouseMoveEvent || !this.data) {
        return;
      }
      var elementRect = this.tipBox.node().getBoundingClientRect();
      var cvsRect = this.getCanvasRect(elementRect);

      var elementX = this.mouseMoveEvent.pageX - elementRect.left;
      var elementY = this.mouseMoveEvent.pageY - elementRect.top;

      var xDate = this.xScale.invert(elementX);

      var i = this.bisect(this.xAxisData, xDate);

      var d0 = this.xAxisData[i - 1];
      var d1 = this.xAxisData[i];

      // work out which date value is closest to the mouse
      this.selectedValueIndex = (!d0 || (xDate - d0[0]) > (d1[0] - xDate)) ? i : i-1;

      this.svg.select(".tip-line")
        .style("opacity", "1")
        .attr("d", function () {
          var idx = this.selectedValueIndex;
          var d = "M" + this.xScale(this.xAxisData[idx][0]) + "," + elementRect.height;
          d += " " + this.xScale(this.xAxisData[idx][0]) + ", 0";
          return d;
        }.bind(this));

      var circlesPerLine =
          this.tipLineWrap.selectAll('.circle-per-line')
          .data(this.data.filter(this.filterDisabled.bind(this)))
          .style('opacity', function (d) {
            var idx = this.selectedValueIndex;
            return (d.values.length && !d.disabled && !isNaN(d.values[idx] && d.values[idx][1])) ? 1 : 0;
          }.bind(this));

      circlesPerLine.join(function (enter) {
        enter
          .append("circle")
          .attr('class', 'circle-per-line')
          .attr("r", 5)
          .call(this.drawCircle.bind(this));
      }.bind(this), function (update) {
        update
          .transition()
          .duration(0)
          .call(this.drawCircle.bind(this));
      }.bind(this));

      if (!this.disableTooltipFlag) {
        var tooltipRows = this.tip
            .style('display', 'block')
            .style('left', this.mouseMoveEvent.pageX + 20 + "px")
            .style('top', this.mouseMoveEvent.pageY - 40 + "px")
            .selectAll(".charts-tooltip-row")
            .data(this.data.filter(this.filterDisabled.bind(this)));

        tooltipRows.join(function (enter) {
          enter
            .append("div")
            .attr('class', 'charts-tooltip-row')
            .html(this.updateLabelRow.bind(this));
        }.bind(this), function (update) {
          update
            .html(this.updateLabelRow.bind(this))
            .transition()
            .duration(0);
        }.bind(this))
      }
    }

    function drawLegends() {
      //Legends
      this.legendsWrap =
        this.rootEl.append("div").attr("class", "legends-wrap")
        .append("div").attr("class", "charts-filter-icon");

      this.getLegends()
        .data(this.data)
        .join(function (enter) {
          enter
            .append("div")
            .attr('class', 'legends')
            .html(getLegendsHtml.bind(this));
        }.bind(this), function (update) {
          update
            .html(getLegendsHtml.bind(this))
            .transition().duration(0);
        }.bind(this));

      this.clickCB = this.getLegends().nodes().map(function (node, i) {
        var cb = function () {
          this.toggleLine(i);
          this.rootEl.dispatch('toggleLegend', {detail: {index: i}});
        }.bind(this);
        angular.element(node).on('click', cb);
        return cb;
      }.bind(this));
    }

    function getLegendsHtml(line, i) {
      return "<i style='background-color:" + this.getLineColor(line, i) + "'></i>" +
        "<span>" + line.key + "</span>";
    }

    function updateLabelRow(line, i) {
      var idx = this.selectedValueIndex;
      if (!(line.values[idx] && line.values[idx].length) || line.disabled) {
        return;
      }
      return "<span><i style='background-color:" + this.getLineColor(line, i) + "'></i>" +
        "<span class='charts-tooltip-key'>" + line.key + "</span></span>" +
        "<span class='bold'>" + ((!line.values[idx] || line.values[idx][1] == undefined) ? "-" :
                    this.cht.tooltip.valueFormatter(line.values[idx][1], line.unit)) + "</span>";
    }

    function updateCirclePosition(pipe, idx) {
      return pipe.attr("transform", function (line) {
        if (line.values[idx] && line.values[idx].length && !isNaN(line.values[idx][1])) {
          return this.getTransform(this.xScale(line.values[idx][0]),
                                   this.yScale[line.yAxis](line.values[idx][1]));
        }
      }.bind(this))
    }

    function hideTooltip() {
      this.mouseMoveEvent = false;
      this.svg.selectAll(".tip-line").style("opacity", 0);
      this.svg.selectAll(".circle-per-line").style("opacity", 0);
      this.tip.style("display", "none");
    }

    function disableTooltip(flag) {
      this.disableTooltipFlag = flag;
    }

    function setMouseMoveEvent(e) {
      this.mouseMoveEvent = e;
    }

    return {
      mdD3: mnD3,
      mnD3Focus: mnD3Focus,
      mnD3Tooltip: mnD3Tooltip,
    };
  }
})();
