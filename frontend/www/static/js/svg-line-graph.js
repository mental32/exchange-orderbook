function createGraph({ data, margin }) {
    const height = 106

    const container = document.getElementById('graph-container');
    const width = container.clientWidth - margin.left - margin.right;

    // Clear previous content
    container.innerHTML = '';

    // Create SVG element
    const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
    svg.setAttribute('width', '100%');
    svg.setAttribute('height', height + margin.top + margin.bottom);
    svg.setAttribute('viewBox', `0 0 ${width + margin.left + margin.right} ${height + margin.top + margin.bottom}`);
    container.appendChild(svg);

    // Create graph group
    const g = document.createElementNS('http://www.w3.org/2000/svg', 'g');
    g.setAttribute('transform', `translate(${margin.left},${margin.top})`);
    svg.appendChild(g);

    // Handle edge cases
    if (data.length === 0) {
        const noDataText = document.createElementNS('http://www.w3.org/2000/svg', 'text');
        noDataText.setAttribute('class', 'no-data-text');
        noDataText.setAttribute('x', width / 2);
        noDataText.setAttribute('y', height / 2);
        noDataText.textContent = 'No data available';
        g.appendChild(noDataText);
        return;
    }

    if (data.length === 1) {
        const singlePoint = document.createElementNS('http://www.w3.org/2000/svg', 'circle');
        singlePoint.setAttribute('cx', width / 2);
        singlePoint.setAttribute('cy', height / 2);
        singlePoint.setAttribute('r', '4');
        singlePoint.setAttribute('fill', '#00FF00');
        g.appendChild(singlePoint);

        const pointText = document.createElementNS('http://www.w3.org/2000/svg', 'text');
        pointText.setAttribute('class', 'no-data-text');
        pointText.setAttribute('x', width / 2);
        pointText.setAttribute('y', height / 2 - 20);
        pointText.textContent = `${data[0].date}: $${data[0].price}`;
        g.appendChild(pointText);
        return;
    }

    // X and Y scales
    const xScale = (index) => (index / (data.length - 1)) * width;
    const yScale = (price) => height - ((price - Math.min(...data.map(d => d.price))) / (Math.max(...data.map(d => d.price)) - Math.min(...data.map(d => d.price)))) * height;

    // Create smooth line function
    function smoothLine(points) {
        if (points.length < 2) return '';

        let path = `M ${points[0][0]} ${points[0][1]}`;

        for (let i = 0; i < points.length - 1; i++) {
            const x1 = points[i][0];
            const y1 = points[i][1];
            const x2 = points[i + 1][0];
            const y2 = points[i + 1][1];

            const cx1 = (x1 + x2) / 2;
            const cy1 = y1;
            const cx2 = (x1 + x2) / 2;
            const cy2 = y2;

            path += ` C ${cx1} ${cy1}, ${cx2} ${cy2}, ${x2} ${y2}`;
        }

        return path;
    }

    // Generate points for the smooth line
    const points = data.map((d, i) => [xScale(i), yScale(d.price)]);
    const smoothPath = smoothLine(points);

    // Create gradient for area
    const areaGradient = document.createElementNS('http://www.w3.org/2000/svg', 'linearGradient');
    areaGradient.setAttribute('id', 'area-gradient');
    areaGradient.setAttribute('gradientUnits', 'userSpaceOnUse');
    areaGradient.setAttribute('x1', '0');
    areaGradient.setAttribute('y1', '0');
    areaGradient.setAttribute('x2', '0');
    areaGradient.setAttribute('y2', height);

    const areaStop1 = document.createElementNS('http://www.w3.org/2000/svg', 'stop');
    areaStop1.setAttribute('offset', '0%');
    areaStop1.setAttribute('stop-color', 'rgba(0, 255, 0, 0.2)');

    const areaStop2 = document.createElementNS('http://www.w3.org/2000/svg', 'stop');
    areaStop2.setAttribute('offset', '5%');
    areaStop2.setAttribute('stop-color', 'rgba(0, 255, 0, 0.05)');

    const areaStop3 = document.createElementNS('http://www.w3.org/2000/svg', 'stop');
    areaStop3.setAttribute('offset', '100%');
    areaStop3.setAttribute('stop-color', 'rgba(0, 255, 0, 0.005)');

    areaGradient.appendChild(areaStop1);
    areaGradient.appendChild(areaStop2);
    areaGradient.appendChild(areaStop3);
    svg.appendChild(areaGradient);

    // Create area under the line
    const area = document.createElementNS('http://www.w3.org/2000/svg', 'path');
    area.setAttribute('d', `${smoothPath} L ${width} ${height} L 0 ${height} Z`);
    area.setAttribute('fill', 'url(#area-gradient)');
    g.appendChild(area);

    // Create smooth line (bright version)
    const brightPath = document.createElementNS('http://www.w3.org/2000/svg', 'path');
    brightPath.setAttribute('d', smoothPath);
    brightPath.setAttribute('fill', 'none');
    brightPath.setAttribute('stroke', '#00FF00');
    brightPath.setAttribute('stroke-width', '2');
    g.appendChild(brightPath);

    // Create smooth line (dark version)
    const darkPath = document.createElementNS('http://www.w3.org/2000/svg', 'path');
    darkPath.setAttribute('d', smoothPath);
    darkPath.setAttribute('fill', 'none');
    darkPath.setAttribute('stroke', '#006400');
    darkPath.setAttribute('stroke-width', '2');
    g.appendChild(darkPath);

    // Create clip path for dark line and circles
    const clipPath = document.createElementNS('http://www.w3.org/2000/svg', 'clipPath');
    clipPath.setAttribute('id', 'dark-line-clip');
    const clipRect = document.createElementNS('http://www.w3.org/2000/svg', 'rect');
    clipRect.setAttribute('x', '0');
    clipRect.setAttribute('y', '0');
    clipRect.setAttribute('width', '0');
    clipRect.setAttribute('height', height);
    clipPath.appendChild(clipRect);
    svg.appendChild(clipPath);

    darkPath.setAttribute('clip-path', 'url(#dark-line-clip)');

    // Add axis lines
    const xAxis = document.createElementNS('http://www.w3.org/2000/svg', 'line');
    xAxis.setAttribute('class', 'axis-line');
    xAxis.setAttribute('x1', 0);
    xAxis.setAttribute('y1', height);
    xAxis.setAttribute('x2', width);
    xAxis.setAttribute('y2', height);
    g.appendChild(xAxis);

    // const yAxis = document.createElementNS('http://www.w3.org/2000/svg', 'line');
    // yAxis.setAttribute('class', 'axis-line');
    // yAxis.setAttribute('x1', 0);
    // yAxis.setAttribute('y1', 0);
    // yAxis.setAttribute('x2', 0);
    // yAxis.setAttribute('y2', height);
    // g.appendChild(yAxis);

    // Add graph title
    const title = document.createElementNS('http://www.w3.org/2000/svg', 'text');
    title.setAttribute('class', 'graph-title');
    title.setAttribute('x', width / 2);
    title.setAttribute('y', -margin.top / 2);
    title.setAttribute('text-anchor', 'middle');
    title.textContent = 'Price Trend';
    g.appendChild(title);

    // Create groups for bright and dark circles
    const brightCirclesGroup = document.createElementNS('http://www.w3.org/2000/svg', 'g');
    g.appendChild(brightCirclesGroup);

    const darkCirclesGroup = document.createElementNS('http://www.w3.org/2000/svg', 'g');
    darkCirclesGroup.setAttribute('clip-path', 'url(#dark-line-clip)');
    g.appendChild(darkCirclesGroup);

    // Add data points (circles)
    // data.forEach((d, i) => {
    //     const brightCircle = document.createElementNS('http://www.w3.org/2000/svg', 'circle');
    //     brightCircle.setAttribute('cx', xScale(i));
    //     brightCircle.setAttribute('cy', yScale(d.price));
    //     brightCircle.setAttribute('r', '4');
    //     brightCircle.setAttribute('fill', '#00FF00');
    //     brightCirclesGroup.appendChild(brightCircle);

    //     const darkCircle = document.createElementNS('http://www.w3.org/2000/svg', 'circle');
    //     darkCircle.setAttribute('cx', xScale(i));
    //     darkCircle.setAttribute('cy', yScale(d.price));
    //     darkCircle.setAttribute('r', '4');
    //     darkCircle.setAttribute('fill', '#006400');
    //     darkCirclesGroup.appendChild(darkCircle);
    // });

    // Create hover elements
    const hoverGroup = document.createElementNS('http://www.w3.org/2000/svg', 'g');
    hoverGroup.style.display = 'none';
    g.appendChild(hoverGroup);

    const hoverLine = document.createElementNS('http://www.w3.org/2000/svg', 'line');
    hoverLine.setAttribute('class', 'hover-line');
    hoverGroup.appendChild(hoverLine);

    const hoverBox = document.createElementNS('http://www.w3.org/2000/svg', 'rect');
    hoverBox.setAttribute('class', 'hover-box');
    hoverBox.setAttribute('width', '120');
    hoverBox.setAttribute('height', '40');
    hoverGroup.appendChild(hoverBox);

    const hoverText = document.createElementNS('http://www.w3.org/2000/svg', 'text');
    hoverText.setAttribute('class', 'hover-text');
    hoverGroup.appendChild(hoverText);

    // Add interactivity
    svg.addEventListener('mousemove', (event) => {
        const rect = svg.getBoundingClientRect();
        const mouseX = event.clientX - rect.left - margin.left;

        if (mouseX >= 0 && mouseX <= width) {
            clipRect.setAttribute('width', width - mouseX);
            clipRect.setAttribute('x', mouseX);

            const index = Math.round((mouseX / width) * (data.length - 1));
            const dataPoint = data[index];

            hoverLine.setAttribute('x1', mouseX);
            hoverLine.setAttribute('y1', 0);
            hoverLine.setAttribute('x2', mouseX);
            hoverLine.setAttribute('y2', height);

            const boxX = Math.min(Math.max(mouseX - 60, 0), width - 120);
            const boxY = 0;  // Fixed to the top
            hoverBox.setAttribute('x', boxX);
            hoverBox.setAttribute('y', boxY);

            hoverText.setAttribute('x', boxX + 5);
            hoverText.setAttribute('y', boxY + 20);
            hoverText.innerHTML = `Date: ${dataPoint.date}<tspan x="${boxX + 5}" dy="20">Price: $${dataPoint.price}</tspan>`;

            hoverGroup.style.display = 'block';
        }
    });

    svg.addEventListener('mouseleave', () => {
        clipRect.setAttribute('width', '0');
        clipRect.setAttribute('x', width);  // Move clip to the right edge
        hoverGroup.style.display = 'none';
    });
}

// Initial creation of the graph
createGraph({
    data: [
        { date: '2023-01-01', price: 100 },
        { date: '2023-02-01', price: 120 },
        { date: '2023-03-01', price: 110 },
        { date: '2023-04-01', price: 130 },
        { date: '2023-05-01', price: 125 },
        { date: '2023-06-01', price: 140 }
    ],
    margin: { top: 0, right: 0, bottom: 0, left: 0 }
});

// Redraw the graph on window resize
window.addEventListener('resize', createGraph);