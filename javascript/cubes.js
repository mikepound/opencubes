class Cube {
  val = 0; // 6 bits, each bit is 1 if there is a cube in that direction
  to = [null, null, null, null, null, null];
  pos = 0; // The position of the cube (= x + 100y + 10000z)
  temp = 0; // used in some algorithms
}

class Polycube {
  cubes = { 0: new Cube() };
  temp = 0;
  n = 1;

  add(pos) {
    const cube = new Cube();
    cube.pos = pos;
    this.cubes[pos] = cube;
    this.n++;
    this.__canonicalInfo = null;

    directions.forEach(i => {
      const pos2 = pos + directionCost[i];
      const cube2 = this.cubes[pos2];
      if (!cube2) return;
      cube.to[i] = cube2;
      cube.val += 1 << i;
      cube2.to[i ^ 1] = cube;
      cube2.val += 1 << (i ^ 1);
    });
  }

  remove(pos) {
    this.cubes[pos].to.forEach((cube, i) => {
      if (!cube) return;
      cube.to[i ^ 1] = null;
      cube.val -= 1 << (i ^ 1);
    });
    delete this.cubes[pos];
    this.n--;
    this.__canonicalInfo = null;
  }

  temporaryAdd(pos, callback) {
    const canonicalInfo = this.__canonicalInfo;
    this.add(pos);
    callback();
    this.remove(pos);
    this.__canonicalInfo = canonicalInfo;
  }

  temporaryRemove(pos, callback) {
    const canonicalInfo = this.__canonicalInfo;
    this.remove(pos);
    callback();
    this.add(pos);
    this.__canonicalInfo = canonicalInfo;
  }

  __toBuffer(rootCube, rotationIndex) {
    // computes the encoding of a cube given a root cube and a rotation
    const rotation = rotations[rotationIndex];
    const cubes = [rootCube];
    rootCube.temp = ++this.temp;
    let i = 0;
    while (cubes.length < this.n) {
      rotation.forEach(j => {
        const cube = cubes[i].to[j];
        if (cube && cube.temp !== this.temp) {
          cube.temp = this.temp;
          cubes.push(cube);
        }
      });
      i++;
    }
    this.__lastPos = cubes[this.n - 1].pos;
    return Buffer.from(cubes.slice(0, i).map(cube => rotationTable[cube.val][rotationIndex]));
  }

  __maximumVertexValues() {
    return Object.values(this.cubes).map(cube => maximumValue[cube.val]).sort();
  }

  canonicalInfo() {
    return this.__canonicalInfo || this.__makeCanonicalInfo();
  }
  __canonicalInfo = null; // memoize this value
  __makeCanonicalInfo() {
    // computes an object containing
    //   - the canonical string (buffer)
    //   - a set of last cubes (there can be more than one if there is symmetry)
    //   - the degree of each cube, sorted
    const maximumVertexValues = this.__maximumVertexValues();
    const maximumVertexValue = maximumVertexValues[this.n - 1];
    let max = { buffer: Buffer.from([]), lastPositions: new Set(), verticesMaxValues: maximumVertexValues };
    Object.values(this.cubes).forEach(cube => {
      if (maximumValue[cube.val] === maximumVertexValue) {
        maximumRotations[cube.val].forEach(rotationIndex => {
          const buffer = this.__toBuffer(cube, rotationIndex);
          if (max.buffer.compare(buffer) < 0) {
            max.buffer = buffer;
            max.lastPositions = new Set();
            max.lastPositions.add(this.__lastPos);
          } else if (max.buffer.compare(buffer) === 0) {
            max.lastPositions.add(this.__lastPos);
          }
        });
      }
    });
    this.__canonicalInfo = max;
    return max;
  }

  toCanonicalString() {
    return `${this.canonicalInfo().buffer.toString('hex')}\0`;
  }

  __equals(canonicalInfo) {
    if (this.__canonicalInfo) return this.__canonicalInfo.buffer.equals(canonicalInfo.buffer);

    const maximumVertexValues = this.__maximumVertexValues();
    if (maximumVertexValues.some((val, i) => val !== canonicalInfo.verticesMaxValues[i])) return false;

    const maximumVertexValue = maximumVertexValues[this.n - 1];
    return Object.values(this.cubes).some(cube => {
      if (maximumValue[cube.val] === maximumVertexValue) {
        return maximumRotations[cube.val].some(rotationIndex => {
          const buffer = this.__toBuffer(cube, rotationIndex);
          if (buffer.equals(canonicalInfo.buffer)) return true;
        });
      }
    });
  }

  extend(n, callback) {
    // extend this to size n, calling callback() on each new polycube
    if (this.n === n) return callback(this.toCanonicalString());

    const seenPos = new Set();
    const seenInfo = [];
    const info = this.canonicalInfo();
    Object.values(this.cubes).forEach(cube => {
      directionCost.forEach((cost) => {
        // for each cube and each direction,
        const pos = cube.pos + cost;
        if (seenPos.has(pos) || this.cubes[pos]) return;
        seenPos.add(pos);
        // if we havent considered this position to add a cube,

        this.temporaryAdd(pos, () => {
          // Add a cube at pos
          const info2 = this.canonicalInfo();
          if (seenInfo.some(seen => seen.buffer.equals(info2.buffer))) return;

          seenInfo.push(info2);
          if (info2.lastPositions.has(pos)) return this.extend(n, callback);

          let isEqual;
          const lastPosition = info2.lastPositions.values().next().value
          this.temporaryRemove(lastPosition, () => {
            // Remove the last cube and see if it is equal to the original cube
            isEqual = this.__equals(info);
          });
          if (isEqual) this.extend(n, callback);
        });
      });
    });
  }
}

// Some constants that we will compute once and use above:
const directions = [0, 1, 2, 3, 4, 5];
const directionCost = [1, -1, 100, -100, 10000, -10000]; // +x, -x, +y, -y, +z, -z
const rotations = [
  [0,1,2,3,4,5], [0,1,3,2,5,4], [0,1,4,5,3,2], [0,1,5,4,2,3],
  [1,0,2,3,5,4], [1,0,3,2,4,5], [1,0,4,5,2,3], [1,0,5,4,3,2],
  [2,3,0,1,5,4], [2,3,1,0,4,5], [2,3,4,5,0,1], [2,3,5,4,1,0],
  [3,2,0,1,4,5], [3,2,1,0,5,4], [3,2,4,5,1,0], [3,2,5,4,0,1],
  [4,5,0,1,2,3], [4,5,1,0,3,2], [4,5,2,3,1,0], [4,5,3,2,0,1],
  [5,4,0,1,3,2], [5,4,1,0,2,3], [5,4,2,3,0,1], [5,4,3,2,1,0],
];

const values = range(64);
const rotationTable = values.map(value => rotations.map(rotation => rotateValue(value, rotation)));
const maximumValue = rotationTable.map(row => row.reduce((acc, cur) => Math.max(acc, cur)));
const maximumRotations = values.map(value => rotations.filter(rotation => maximumValue[value] === rotateValue(value, rotation)).map(rotation => rotations.findIndex((rot) => rot === rotation)));

function range(n) {
  return Array.from({ length: n }, (_, i) => i);
}
function toBitArray(val) {
  // eg. 53 (= 110101b) => [1,0,1,0,1,1]
  return [0,1,2,3,4,5].map(x => ((val & (1 << x)) ? 1 : 0));
}
function toValue(bitArray) {
  return bitArray.reduce((acc, cur, i) => acc + cur * (1 << i), 0);
}
function rotate(bitArray, rotation) {
  return rotation.map(i => bitArray[i]);
}
function rotateValue(value, rotation) {
  return toValue(rotate(toBitArray(value), rotation));
}



// main code:

const p = new Polycube();
const n = Number(process.argv[2]) || 8;

let count = 0;
const save = (encodedCube) => {
  // console.log(encodedCube); // or write to file
  count++;
}

console.time('time');
p.extend(n, save);
console.log(`Found ${count} polycubes of size ${n}`);
console.timeEnd('time');

